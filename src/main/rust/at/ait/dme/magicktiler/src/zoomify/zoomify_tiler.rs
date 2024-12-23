use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use log::{debug, error, info};

use crate::image::ImageProcessor;
use crate::magick_tiler::{BaseMagickTiler, MagickTiler, TilingError};
use crate::stripe::{Orientation, Stripe};
use crate::tile_set_info::TileSetInfo;

/// A tiler that implements the Zoomify tiling scheme.
///
/// The Zoomify tiling scheme arranges tiles in the following folder/file
/// structure:
/// /tileset-root/TileGroup[group-no]/[zoomlevel]-[column]-[row].jpg
///
/// The highest-resolution zoom level has the highest number. Column/row
/// numbering of tiles starts top/left, counting direction is right/downwards.
///
/// Zoomify allows irregularly sized tiles on the border: I.e. the tiles in the
/// last (=right-most) column and in the last (=bottom-most) row do not need to
/// be rectangular.
pub struct ZoomifyTiler {
    base: BaseMagickTiler,
}

pub const MAX_TILES_PER_GROUP: i32 = 256;
pub const TILEGROUP: &str = "TileGroup";
const METADATA_TEMPLATE: &str = r#"<IMAGE_PROPERTIES WIDTH="@width@" HEIGHT="@height@" NUMTILES="@numtiles@" NUMIMAGES="1" VERSION="1.8" TILESIZE="@tilesize@" />"#;

impl ZoomifyTiler {
    pub fn new() -> Self {
        Self {
            base: BaseMagickTiler::new(),
        }
    }

    fn generate_zoomify_tiles(
        &self,
        stripe: &Stripe,
        zoomlevel: i32,
        x_tiles: i32,
        start_idx: i32,
        row_number: i32,
    ) -> Result<(), TilingError> {
        let filename_pattern = self.base.tileset_root_dir().unwrap().join("tmp-%d.jpg");

        self.base.processor().crop(
            stripe.image_file(),
            &filename_pattern,
            self.base.tile_width(),
            self.base.tile_height(),
        )?;

        // Rename result files
        for idx in 0..x_tiles {
            let tile_group = (start_idx + idx) / MAX_TILES_PER_GROUP;
            let tile_group_dir = self
                .base
                .tileset_root_dir()
                .unwrap()
                .join(format!("{}{}", TILEGROUP, tile_group));

            if !tile_group_dir.exists() {
                fs::create_dir_all(&tile_group_dir)?;
            }

            let old_name = filename_pattern.with_file_name(format!("tmp-{}.jpg", idx));
            let new_name = tile_group_dir.join(format!(
                "{}-{}-{}.jpg",
                zoomlevel,
                idx % x_tiles,
                row_number
            ));

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
        match stripe2 {
            None => Ok(stripe1.shrink(
                target_file,
                self.base.processor().get_image_processing_system(),
            )?),
            Some(s2) => Ok(stripe1.merge(
                s2,
                target_file,
                self.base.processor().get_image_processing_system(),
            )?),
        }
    }

    fn generate_image_properties_xml(&self, info: &TileSetInfo) -> Result<(), TilingError> {
        let metadata = METADATA_TEMPLATE
            .replace("@width@", &info.image_width().to_string())
            .replace("@height@", &info.image_height().to_string())
            .replace("@numtiles@", &info.total_number_of_tiles().to_string())
            .replace("@tilesize@", &info.tile_height().to_string());

        if let Some(root_dir) = self.base.tileset_root_dir() {
            let metadata_path = root_dir.join("ImageProperties.xml");
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

impl MagickTiler for ZoomifyTiler {
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
            "Generating Zoomify tiles for file {}: {}x{}, {}x{} basetiles, {} zoom levels, {} tiles total",
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
        let base_stripes = self.base.stripe_image(
            image,
            Orientation::Horizontal,
            info.number_of_y_tiles(0),
            info.image_width(),
            self.base.tile_height(),
            &format!("{}-0-", base_name),
        )?;

        // Step 2 - tile base image stripes
        debug!("Tiling level 1");
        let zoomlevel_start_idx =
            info.total_number_of_tiles() - info.number_of_x_tiles(0) * info.number_of_y_tiles(0);
        let mut offset = zoomlevel_start_idx;

        for (i, stripe) in base_stripes.iter().enumerate() {
            self.generate_zoomify_tiles(
                stripe,
                info.zoom_levels() as i32 - 1,
                info.number_of_x_tiles(0),
                offset,
                i as i32,
            )?;
            offset += info.number_of_x_tiles(0);
        }

        // Step 3 - compute the pyramid
        let mut level_beneath = base_stripes;
        let mut this_level = Vec::new();
        let mut zoomlevel_start_idx = zoomlevel_start_idx;

        for i in 1..info.zoom_levels() {
            debug!("Tiling level {}", i + 1);
            zoomlevel_start_idx -= info.number_of_x_tiles(i) * info.number_of_y_tiles(i);
            let mut offset = zoomlevel_start_idx;

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
                self.generate_zoomify_tiles(
                    this_level.last().unwrap(),
                    info.zoom_levels() as i32 - i as i32 - 1,
                    info.number_of_x_tiles(i),
                    offset,
                    j as i32,
                )?;
                offset += info.number_of_x_tiles(i);
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

        // Step 4 - generate ImageProperties.xml
        self.generate_image_properties_xml(&info)?;

        // Step 5 (optional) - generate OpenLayers preview
        if self.base.generate_preview() {
            self.base.generate_preview(&info)?;
        }

        info!("Took {} ms", start_time.elapsed().as_millis());
        Ok(info)
    }
}
