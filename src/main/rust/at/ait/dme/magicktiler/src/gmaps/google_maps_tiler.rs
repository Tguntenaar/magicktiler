use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use log::{debug, error, info};

use crate::image::ImageProcessor;
use crate::magick_tiler::{BaseMagickTiler, MagickTiler, TilingError};
use crate::stripe::{Orientation, Stripe};
use crate::tile_set_info::TileSetInfo;

/// A tiler that implements the Google Maps tiling scheme.
///
/// The Google Maps tiling scheme arranges tiles in the following folder/file structure:
/// /tileset-root/[zoomlevel]-[column]-[row].jpg (or .png)
///
/// The highest-resolution zoom level has the highest number. Column/row
/// numbering of tiles starts top/left, counting direction is right/downwards.
pub struct GoogleMapsTiler {
    base: BaseMagickTiler,
}

pub const METADATA_FILE: &str = "gmap_tileset.info";

impl GoogleMapsTiler {
    pub fn new() -> Self {
        Self {
            base: BaseMagickTiler::new(),
        }
    }

    fn stripe_base_image(&self, info: &mut TileSetInfo) -> Result<Vec<Stripe>, TilingError> {
        let prefix = format!(
            "{}-0-",
            info.image_file().file_stem().unwrap().to_string_lossy()
        );

        let (orientation, stripe_width, stripe_height, canvas_width, canvas_height, stripes) =
            if info.image_width() > info.image_height() {
                let stripe_width = self.base.tile_width();
                let canvas_width = stripe_width;
                let stripes = info.image_width() / stripe_width;
                let stripe_height = info.image_height();
                // square the image
                let canvas_height = info.image_width();
                info.set_dimension(canvas_height, canvas_height);
                (
                    Orientation::Vertical,
                    stripe_width,
                    stripe_height,
                    canvas_width,
                    canvas_height,
                    stripes,
                )
            } else {
                let stripe_height = self.base.tile_height();
                let canvas_height = stripe_height;
                let stripes = info.image_height() / stripe_height;
                let stripe_width = info.image_width();
                // square the image
                let canvas_width = info.image_height();
                info.set_dimension(canvas_width, canvas_width);
                (
                    Orientation::Horizontal,
                    stripe_width,
                    stripe_height,
                    canvas_width,
                    canvas_height,
                    stripes,
                )
            };

        self.base.stripe_image(
            info.image_file(),
            orientation,
            stripes,
            stripe_width,
            stripe_height,
            canvas_width,
            canvas_height,
            "center",
            &prefix,
        )
    }

    fn create_stripes_for_next_zoom_level(
        &self,
        stripes: &[Stripe],
        base_file_name: &str,
        z: i32,
    ) -> Result<Vec<Stripe>, TilingError> {
        let base_name = self
            .base
            .working_directory()
            .join(base_file_name)
            .with_extension("");

        let mut next_level = Vec::new();
        for i in 0..((stripes.len() as f64 / 2.0).ceil() as usize) {
            let target_stripe = base_name.with_extension(format!("{}-{}.tif", z, i));
            let stripe1 = &stripes[i * 2];
            let stripe2 = if i * 2 + 1 < stripes.len() {
                Some(&stripes[i * 2 + 1])
            } else {
                None
            };

            // we should always have an even number of stripes
            if let Some(s2) = stripe2 {
                let result = stripe1.merge(
                    s2,
                    &target_stripe,
                    self.base.processor().get_image_processing_system(),
                )?;
                next_level.push(result);
            }
        }
        Ok(next_level)
    }

    fn resize_base_image(
        &self,
        image: &Path,
        info: &TileSetInfo,
        target_file_name: &Path,
    ) -> Result<TileSetInfo, TilingError> {
        // find the closest multiple of 256 and the power of 2
        let max_dim = info.image_width().max(info.image_height());
        let mut new_max_dim = 0;
        let mut prev_max_dim = 0;
        for pow in 0.. {
            prev_max_dim = new_max_dim;
            new_max_dim = 256 * (2i32.pow(pow));
            if new_max_dim > max_dim {
                break;
            }
        }
        let new_max_dim = if (max_dim - prev_max_dim).abs() < (max_dim - new_max_dim).abs() {
            prev_max_dim
        } else {
            new_max_dim
        };

        // calculate the new height and width
        let (new_height, new_width) = if max_dim == info.image_height() {
            let new_height = new_max_dim;
            let new_width = new_height
                * ((info.image_width() as f32 / info.image_height() as f32).ceil() as i32);
            (new_height, new_width)
        } else {
            let new_width = new_max_dim;
            let new_height = new_width
                * ((info.image_width() as f32 / info.image_height() as f32).ceil() as i32);
            (new_height, new_width)
        };

        self.base
            .processor()
            .resize(image, target_file_name, new_width, new_height)?;

        TileSetInfo::new(
            target_file_name,
            self.base.tile_width(),
            self.base.tile_height(),
            self.base.processor(),
        )
    }

    fn generate_preview(&self, info: &TileSetInfo) -> Result<(), TilingError> {
        let template = include_str!("gmaps-template.html");
        let html = template
            .replace(
                "@title@",
                &info.image_file().file_name().unwrap().to_string_lossy(),
            )
            .replace("@zoomlevels@", &info.zoom_levels().to_string())
            .replace("@maxzoom@", &(info.zoom_levels() - 1).to_string())
            .replace(
                "@tilesetpath@",
                &self
                    .base
                    .tileset_root_dir()
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/"),
            )
            .replace("@ext@", &info.tile_format().extension());

        self.base.write_html_preview(&html)
    }
}

impl MagickTiler for GoogleMapsTiler {
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
            "Generating Google Map tiles for file {}",
            image.file_name().unwrap().to_string_lossy()
        );

        let mut all_stripes = Vec::new();
        let mut info = info;

        debug!("Resizing base image");
        // Step 1: resize to the closest 256*n^2
        let src = self.base.tileset_root_dir().unwrap().join(format!(
            "gmapbase.{}",
            self.base.processor().get_image_format().extension()
        ));
        info = self.resize_base_image(image, &info, &src)?;

        debug!("Striping base image");
        // Step 2: cut the image into stripes, thereby creating a squared result image
        let mut stripes = self.stripe_base_image(&mut info)?;
        all_stripes.extend(stripes.clone());

        for z in (0..info.zoom_levels()).rev() {
            debug!("Tiling level {}", z);
            // Step 3: create the tiles for this zoom level
            let tile_base = self.base.tileset_root_dir().unwrap().join(z.to_string());

            for (s, stripe) in stripes.iter().enumerate() {
                let filename_pattern = tile_base.with_extension(format!(
                    "_%d.{}",
                    self.base.processor().get_image_format().extension()
                ));

                self.base.processor().crop(
                    stripe.image_file(),
                    &filename_pattern,
                    self.base.tile_width(),
                    self.base.tile_height(),
                )?;

                let tiles = if stripe.orientation() == Orientation::Horizontal {
                    stripe.width() / self.base.tile_width()
                } else {
                    stripe.height() / self.base.tile_height()
                };

                for t in 0..tiles {
                    let (column, row) = if stripe.orientation() == Orientation::Horizontal {
                        (t, s as i32)
                    } else {
                        (s as i32, t)
                    };

                    let old_name = filename_pattern.with_extension(format!(
                        "_{}.{}",
                        t,
                        self.base.processor().get_image_format().extension()
                    ));
                    let new_name = tile_base.with_extension(format!(
                        "_{}_{}_.{}",
                        column,
                        row,
                        self.base.processor().get_image_format().extension()
                    ));

                    fs::rename(&old_name, &new_name).map_err(|e| {
                        TilingError::General(format!(
                            "Failed to rename file {}: {}",
                            old_name.display(),
                            e
                        ))
                    })?;
                }
            }

            stripes = self.create_stripes_for_next_zoom_level(
                &stripes,
                &image.file_name().unwrap().to_string_lossy(),
                info.zoom_levels() - z,
            )?;
            all_stripes.extend(stripes.clone());
        }

        // Step 4: optionally create the preview.html
        if self.base.generate_preview() {
            self.generate_preview(&info)?;
        }

        // Step 5: write the metadata file
        let metadata_path = self.base.tileset_root_dir().unwrap().join(METADATA_FILE);
        let metadata = serde_json::to_string(&info)?;
        fs::write(&metadata_path, metadata)?;

        // Clean up
        for stripe in all_stripes {
            if let Err(e) = stripe.delete() {
                error!(
                    "Could not delete stripe {}: {}",
                    stripe.image_file().display(),
                    e
                );
            }
        }

        info!("Took {} ms", start_time.elapsed().as_millis());
        Ok(info)
    }
}
