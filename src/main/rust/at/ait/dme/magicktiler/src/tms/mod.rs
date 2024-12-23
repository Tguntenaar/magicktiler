use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use log::{debug, error, info};

use crate::image::ImageProcessor;
use crate::magick_tiler::{BaseMagickTiler, MagickTiler, TilingError};
use crate::stripe::{Orientation, Stripe};
use crate::tile_set_info::TileSetInfo;

/// A tiler that implements the TMS tiling scheme.
///
/// The TMS tiling scheme arranges tiles in the following folder/file structure:
/// /tileset-root/[zoomlevel]/[column]/[row].jpg (or .png)
///
/// The highest-resolution zoom level has the highest number. Column/row
/// numbering of tiles starts left/bottom, counting direction is upwards/right.
///
/// TMS does NOT allow irregularly sized tiles on the border! Each tile must
/// be rectangular. If the image width/height are not integer multiples of
/// the tilesize, a background-color buffer must be added. TMS mandates this
/// buffer to be added to the TOP and RIGHT of the image!
pub struct TMSTiler {
    base: BaseMagickTiler,
}

const METADATA_TEMPLATE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<TileMap version="1.0.0" tilemapservice="http://tms.osgeo.org/1.0.0">
  <Title>@title@</Title>
  <Abstract></Abstract>
  <SRS></SRS>
  <BoundingBox minx="-@height@.00000000000000" miny="0.00000000000000" maxx="0.00000000000000" maxy="@width@.00000000000000"/>
  <Origin x="-@height@.00000000000000" y="0.00000000000000"/>
  <TileFormat width="@tilewidth@" height="@tileheight@" mime-type="@mimetype@" extension="@ext@"/>
  <TileSets profile="raster">
@tilesets@  </TileSets>
</TileMap>
"#;

const TILESET_TEMPLATE: &str = "    <TileSet href=\"@idx@\" units-per-pixel=\"@unitsPerPixel@.00000000000000\" order=\"@idx@\"/>\n";

impl TMSTiler {
    pub fn new() -> Self {
        let mut base = BaseMagickTiler::new();
        base.set_background_color("#ffffffff".to_string());
        Self { base }
    }

    fn generate_tms_tiles(
        &self,
        stripe: &Stripe,
        info: &TileSetInfo,
        target_dir: &Path,
    ) -> Result<(), TilingError> {
        // Tile the stripe
        let filename_pattern = target_dir
            .join("tmp-%d")
            .with_extension(info.tile_format().extension());

        self.base.processor().crop(
            stripe.image_file(),
            &filename_pattern,
            info.tile_width(),
            info.tile_height(),
        )?;

        // Rename result files
        for i in 0..(stripe.height() / info.tile_height()) {
            let old_name = filename_pattern
                .with_file_name(format!("tmp-{}", i))
                .with_extension(info.tile_format().extension());
            let new_name = filename_pattern
                .with_file_name(format!(
                    "{}",
                    (stripe.height() / info.tile_height()) - i - 1
                ))
                .with_extension(info.tile_format().extension());

            fs::rename(&old_name, &new_name).map_err(|e| {
                TilingError::General(format!(
                    "Failed to rename file {}: {}",
                    old_name.display(),
                    e
                ))
            })?;
        }

        Ok(())
    }

    fn merge_stripes(
        &self,
        stripe1: &Stripe,
        stripe2: Option<&Stripe>,
        target_file: &Path,
    ) -> Result<Stripe, TilingError> {
        let height = if (stripe1.height() / self.base.tile_height()) % 2 != 0 {
            stripe1.height() / 2 + self.base.tile_height() / 2
        } else {
            stripe1.height() / 2
        };

        match stripe2 {
            None => Ok(stripe1.shrink_with_canvas(
                Some(ImageProcessor::GRAVITY_SOUTHWEST),
                self.base.tile_width(),
                height,
                Some("#ffffffff"),
                target_file,
                self.base.processor().get_image_processing_system(),
            )?),
            Some(s2) => Ok(stripe1.merge_with_canvas(
                s2,
                Some(ImageProcessor::GRAVITY_SOUTHWEST),
                self.base.tile_width(),
                height,
                Some("#ffffffff"),
                target_file,
                self.base.processor().get_image_processing_system(),
            )?),
        }
    }

    fn generate_tilemap_resource_xml(&self, info: &TileSetInfo) -> Result<(), TilingError> {
        let mut tilesets = String::new();
        for i in 0..info.zoom_levels() {
            tilesets.push_str(&TILESET_TEMPLATE.replace("@idx@", &i.to_string()).replace(
                "@unitsPerPixel@",
                &(2_i32.pow((info.zoom_levels() - i - 1) as u32)).to_string(),
            ));
        }

        let metadata = METADATA_TEMPLATE
            .replace(
                "@title@",
                &info.image_file().file_name().unwrap().to_string_lossy(),
            )
            .replace("@width@", &info.image_width().to_string())
            .replace("@height@", &info.image_height().to_string())
            .replace("@tilewidth@", &info.tile_width().to_string())
            .replace("@tileheight@", &info.tile_height().to_string())
            .replace("@mimetype@", info.tile_format().mime_type())
            .replace("@ext@", info.tile_format().extension())
            .replace("@tilesets@", &tilesets);

        if let Some(root_dir) = self.base.tileset_root_dir() {
            let metadata_path = root_dir.join("tilemapresource.xml");
            let mut file = File::create(&metadata_path).map_err(|e| {
                TilingError::General(format!(
                    "Error creating metadata file {}: {}",
                    metadata_path.display(),
                    e
                ))
            })?;

            file.write_all(metadata.as_bytes()).map_err(|e| {
                error!("Error writing metadata XML: {}", e);
                TilingError::from(e)
            })?;
        }

        Ok(())
    }
}

impl MagickTiler for TMSTiler {
    fn convert(&mut self, image: &Path) -> Result<TileSetInfo, TilingError> {
        self.base.convert(image)
    }

    fn convert_to(&mut self, image: &Path, target: &Path) -> Result<TileSetInfo, TilingError> {
        self.base.convert_to(image, target)
    }

    fn convert_internal(
        &mut self,
        image: &Path,
        info: TileSetInfo,
    ) -> Result<TileSetInfo, TilingError> {
        let start_time = std::time::Instant::now();
        info!(
            "Generating TMS tiles for file {}: {}x{}, {}x{} basetiles, {} zoom levels, {} tiles total",
            image.file_name().unwrap().to_string_lossy(),
            info.image_width(),
            info.image_height(),
            info.number_of_x_tiles(0),
            info.number_of_y_tiles(0),
            info.zoom_levels(),
            info.total_number_of_tiles()
        );

        let base_name = image.file_stem().unwrap().to_string_lossy().into_owned();

        // Step 1 - stripe the base image
        debug!("Striping base image");
        let canvas_height = info.image_height() + self.base.tile_height()
            - (info.image_height() % self.base.tile_height());

        let base_stripes = self.base.stripe_image(
            image,
            Orientation::Vertical,
            info.number_of_x_tiles(0),
            self.base.tile_width(),
            info.image_height(),
            self.base.tile_width(),
            canvas_height,
            ImageProcessor::GRAVITY_SOUTHWEST,
            &format!("{}-0-", base_name),
        )?;

        // Step 2 - tile base image stripes
        debug!("Tiling level 1");
        let baselayer_dir = self
            .base
            .tileset_root_dir()
            .unwrap()
            .join((info.zoom_levels() - 1).to_string());
        fs::create_dir_all(&baselayer_dir)?;

        for (i, stripe) in base_stripes.iter().enumerate() {
            let target_dir = baselayer_dir.join(i.to_string());
            fs::create_dir_all(&target_dir)?;
            self.generate_tms_tiles(stripe, &info, &target_dir)?;
        }

        // Step 3 - compute the pyramid
        let mut level_beneath = base_stripes;
        let mut this_level = Vec::new();

        for i in 1..info.zoom_levels() {
            debug!("Tiling level {}", i + 1);
            let zoom_level_dir = self
                .base
                .tileset_root_dir()
                .unwrap()
                .join((info.zoom_levels() - i - 1).to_string());
            fs::create_dir_all(&zoom_level_dir)?;

            for j in 0..((level_beneath.len() as f64 / 2.0).ceil() as usize) {
                // Step 3a - merge stripes from level beneath
                let stripe1 = &level_beneath[j * 2];
                let stripe2 = if j * 2 + 1 < level_beneath.len() {
                    Some(&level_beneath[j * 2 + 1])
                } else {
                    None
                };

                let result = self.merge_stripes(
                    stripe1,
                    stripe2,
                    &self
                        .base
                        .working_directory()
                        .join(format!("{}-{}-{}.tif", base_name, i, j)),
                )?;
                this_level.push(result);

                // Step 3b - tile result stripe
                let target_dir = zoom_level_dir.join(j.to_string());
                fs::create_dir_all(&target_dir)?;
                self.generate_tms_tiles(&this_level.last().unwrap(), &info, &target_dir)?;
            }

            for s in &level_beneath {
                s.delete()?;
            }
            level_beneath = this_level;
            this_level = Vec::new();
        }

        for s in &level_beneath {
            s.delete()?;
        }

        // Step 4 - generate tilemapresource.xml
        self.generate_tilemap_resource_xml(&info)?;

        // Step 5 (optional) - generate OpenLayers preview
        if self.base.generate_preview() {
            self.base.generate_preview(&info)?;
        }

        info!("Took {} ms", start_time.elapsed().as_millis());
        Ok(info)
    }
}
