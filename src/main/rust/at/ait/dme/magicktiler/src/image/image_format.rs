use serde::{Deserialize, Serialize};

/// Supported image file formats. Please note that not all tiling schemes
/// may support all image file formats!
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFormat {
    /// JPEG format (image/jpeg, .jpg)
    JPEG,
    /// PNG format (image/png, .png)
    PNG,
    /// TIFF format (image/tiff, .tif)
    TIFF,
}

impl ImageFormat {
    pub fn mime_type(&self) -> &'static str {
        match self {
            ImageFormat::JPEG => "image/jpeg",
            ImageFormat::PNG => "image/png",
            ImageFormat::TIFF => "image/tiff",
        }
    }

    pub fn extension(&self) -> &'static str {
        match self {
            ImageFormat::JPEG => "jpg",
            ImageFormat::PNG => "png",
            ImageFormat::TIFF => "tif",
        }
    }
}
