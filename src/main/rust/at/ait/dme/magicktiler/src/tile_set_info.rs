use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::image::{ImageFormat, ImageInfo, ImageProcessor};

#[derive(Debug, Serialize, Deserialize)]
pub struct TileSetInfo {
    /// Path to the source image file
    image_file: PathBuf,

    /// Width of the source image
    width: i32,

    /// Height of the source image
    height: i32,

    /// Width of a single tile
    tile_width: i32,

    /// Height of a single tile
    tile_height: i32,

    /// Format of the tiles (jpg, png, etc.)
    format: ImageFormat,

    /// Image info
    img_info: ImageInfo,
}

impl TileSetInfo {
    pub fn new(
        image: &Path,
        tile_width: i32,
        tile_height: i32,
        processor: &dyn ImageProcessor,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            image_file: image.to_path_buf(),
            width: 0,
            height: 0,
            tile_width,
            tile_height,
            format: processor.get_image_format(),
            img_info: ImageInfo::new(image, processor.get_image_processing_system())?,
        })
    }

    pub fn image_file(&self) -> &Path {
        &self.image_file
    }

    pub fn image_width(&self) -> i32 {
        self.width
    }

    pub fn image_height(&self) -> i32 {
        self.height
    }

    pub fn set_dimension(&mut self, width: i32, height: i32) {
        self.width = width;
        self.height = height;
    }

    pub fn tile_width(&self) -> i32 {
        self.tile_width
    }

    pub fn tile_height(&self) -> i32 {
        self.tile_height
    }

    pub fn tile_format(&self) -> ImageFormat {
        self.format
    }

    pub fn zoom_levels(&self) -> i32 {
        let max_dim = self.width.max(self.height);
        let max_tiles = (max_dim as f64 / self.tile_width as f64).ceil() as i32;
        (max_tiles as f64).log2().ceil() as i32 + 1
    }

    pub fn number_of_x_tiles(&self, zoom_level: i32) -> i32 {
        let factor = 2i32.pow(zoom_level as u32);
        ((self.width as f64 / factor as f64) / self.tile_width as f64).ceil() as i32
    }

    pub fn number_of_y_tiles(&self, zoom_level: i32) -> i32 {
        let factor = 2i32.pow(zoom_level as u32);
        ((self.height as f64 / factor as f64) / self.tile_height as f64).ceil() as i32
    }

    pub fn total_number_of_tiles(&self) -> i32 {
        let mut total = 0;
        for z in 0..self.zoom_levels() {
            total += self.number_of_x_tiles(z) * self.number_of_y_tiles(z);
        }
        total
    }
}
