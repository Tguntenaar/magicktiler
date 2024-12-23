use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Information about an image file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageInfo {
    file: PathBuf,
    width: i32,
    height: i32,
}

impl ImageInfo {
    pub fn new(file: &Path, system: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            file: file.to_path_buf(),
            width: 0,
            height: 0,
        })
    }

    pub fn file(&self) -> &Path {
        &self.file
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn set_width(&mut self, width: i32) {
        self.width = width;
    }

    pub fn set_height(&mut self, height: i32) {
        self.height = height;
    }
}

impl std::fmt::Display for ImageInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ImageInfo [file={}, width={}, height={}]",
            self.file.display(),
            self.width,
            self.height
        )
    }
}
