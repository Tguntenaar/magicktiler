use crate::image::ImageFormat;
use std::path::Path;

/// Trait for image processing operations
pub trait ImageProcessor {
    /// Get the image processing system being used (e.g., "ImageMagick")
    fn get_image_processing_system(&self) -> &str;

    /// Get the image format being used
    fn get_image_format(&self) -> ImageFormat;

    /// Set the image format to use
    fn set_image_format(&mut self, format: ImageFormat);

    /// Resize an image to the specified dimensions
    fn resize(
        &self,
        src: &Path,
        target: &Path,
        width: i32,
        height: i32,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Crop an image into tiles
    fn crop(
        &self,
        src: &Path,
        target: &Path,
        width: i32,
        height: i32,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Merge two images side by side
    fn merge(
        &self,
        src1: &Path,
        src2: &Path,
        target: &Path,
    ) -> Result<(), Box<dyn std::error::Error>>;

    /// Get the dimensions of an image
    fn get_dimensions(&self, image: &Path) -> Result<(i32, i32), Box<dyn std::error::Error>>;
}
