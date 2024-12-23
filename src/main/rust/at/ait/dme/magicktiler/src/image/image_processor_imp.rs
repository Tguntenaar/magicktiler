use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use super::image_format::ImageFormat;
use super::image_processor::ImageProcessor;

/// Supported image processing systems: GraphicsMagick or ImageMagick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageProcessingSystem {
    GraphicsMagick,
    ImageMagick,
}

/// A concrete implementation of the ImageProcessor trait
#[derive(Debug)]
pub struct ImageProcessorImpl {
    /// The processing system used by this ImageProcessor
    processing_system: ImageProcessingSystem,

    /// The image format this processor will produce as output
    format: ImageFormat,

    /// JPEG compression quality (in case of JPEG image format), default=75
    jpeg_quality: i32,

    /// The default background color for montage operations
    background_color: Option<String>,
}

impl ImageProcessorImpl {
    pub const GRAVITY_CENTER: &'static str = "Center";
    pub const GRAVITY_SOUTHWEST: &'static str = "SouthWest";

    pub fn new(processing_system: ImageProcessingSystem) -> Self {
        Self {
            processing_system,
            format: ImageFormat::JPEG,
            jpeg_quality: 75,
            background_color: None,
        }
    }

    pub fn with_format(processing_system: ImageProcessingSystem, format: ImageFormat) -> Self {
        Self {
            processing_system,
            format,
            jpeg_quality: 75,
            background_color: None,
        }
    }

    pub fn with_background(
        processing_system: ImageProcessingSystem,
        format: ImageFormat,
        background_color: String,
    ) -> Self {
        Self {
            processing_system,
            format,
            jpeg_quality: 75,
            background_color: Some(background_color),
        }
    }

    pub fn with_quality(
        processing_system: ImageProcessingSystem,
        format: ImageFormat,
        background_color: Option<String>,
        jpeg_quality: i32,
    ) -> Self {
        Self {
            processing_system,
            format,
            jpeg_quality,
            background_color,
        }
    }

    fn create_convert_command(&self) -> Command {
        let mut cmd = Command::new(
            if self.processing_system == ImageProcessingSystem::GraphicsMagick {
                "gm"
            } else {
                "convert"
            },
        );
        if self.processing_system == ImageProcessingSystem::GraphicsMagick {
            cmd.arg("convert");
        }
        cmd
    }
}

impl ImageProcessor for ImageProcessorImpl {
    fn get_image_processing_system(&self) -> &str {
        match self.processing_system {
            ImageProcessingSystem::GraphicsMagick => "GraphicsMagick",
            ImageProcessingSystem::ImageMagick => "ImageMagick",
        }
    }

    fn get_image_format(&self) -> ImageFormat {
        self.format
    }

    fn set_image_format(&mut self, format: ImageFormat) {
        self.format = format;
    }

    fn resize(
        &self,
        src: &Path,
        target: &Path,
        width: i32,
        height: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = self.create_convert_command();
        cmd.arg(src)
            .arg("-resize")
            .arg(format!("{}x{}", width, height))
            .arg(target);

        cmd.output().map(|_| ()).map_err(|e| e.into())
    }

    fn crop(
        &self,
        src: &Path,
        target: &Path,
        width: i32,
        height: i32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = self.create_convert_command();
        cmd.arg(src)
            .arg("-crop")
            .arg(format!("{}x{}", width, height))
            .arg("+adjoin")
            .arg(target);

        cmd.output().map(|_| ()).map_err(|e| e.into())
    }

    fn merge(
        &self,
        src1: &Path,
        src2: &Path,
        target: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut cmd = self.create_convert_command();
        if let Some(bg) = &self.background_color {
            cmd.arg("-background").arg(bg);
        }
        cmd.arg(src1).arg(src2).arg("+append").arg(target);

        cmd.output().map(|_| ()).map_err(|e| e.into())
    }

    fn get_dimensions(&self, image: &Path) -> Result<(i32, i32), Box<dyn std::error::Error>> {
        let mut cmd = Command::new(
            if self.processing_system == ImageProcessingSystem::GraphicsMagick {
                "gm"
            } else {
                "identify"
            },
        );
        cmd.arg("identify").arg(image);

        let output = cmd.output()?;
        let output_str = String::from_utf8_lossy(&output.stdout);

        // Parse dimensions from output (format: "image.jpg JPEG 1920x1080+0+0")
        let parts: Vec<&str> = output_str.split_whitespace().collect();
        if parts.len() >= 3 {
            if let Some(dimensions) = parts[2].split('x').next() {
                let dims: Vec<&str> = dimensions.split('x').collect();
                if dims.len() == 2 {
                    let width = dims[0].parse::<i32>()?;
                    let height = dims[1].parse::<i32>()?;
                    return Ok((width, height));
                }
            }
        }

        Err("Failed to parse image dimensions".into())
    }
}
