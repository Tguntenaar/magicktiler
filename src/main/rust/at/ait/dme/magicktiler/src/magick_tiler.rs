use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

use crate::image::{ImageFormat, ImageProcessingSystem, ImageProcessor, ImageProcessorImpl};
use crate::tile_set_info::TileSetInfo;

#[derive(Debug, Error)]
pub enum TilingError {
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("General error: {0}")]
    General(String),
}

impl From<Box<dyn std::error::Error>> for TilingError {
    fn from(err: Box<dyn std::error::Error>) -> Self {
        Self::General(err.to_string())
    }
}

impl From<serde_json::Error> for TilingError {
    fn from(err: serde_json::Error) -> Self {
        Self::General(err.to_string())
    }
}

pub trait MagickTiler {
    fn convert(&mut self, image: &Path) -> Result<TileSetInfo, TilingError>;
    fn convert_to(&mut self, image: &Path, target: &Path) -> Result<TileSetInfo, TilingError>;
    fn convert_internal(
        &mut self,
        image: &Path,
        info: TileSetInfo,
    ) -> Result<TileSetInfo, TilingError>;
}

pub struct BaseMagickTiler {
    pub processor: Box<dyn ImageProcessor>,
    pub tile_width: i32,
    pub tile_height: i32,
    pub generate_preview: bool,
    pub working_directory: Option<PathBuf>,
    pub tileset_root_dir: Option<PathBuf>,
}

impl BaseMagickTiler {
    pub fn new() -> Self {
        Self {
            processor: Box::new(ImageProcessorImpl::new(
                ImageProcessingSystem::GraphicsMagick,
            )),
            tile_width: 256,
            tile_height: 256,
            generate_preview: true,
            working_directory: None,
            tileset_root_dir: None,
        }
    }

    pub fn processor(&self) -> &dyn ImageProcessor {
        self.processor.as_ref()
    }

    pub fn tile_width(&self) -> i32 {
        self.tile_width
    }

    pub fn tile_height(&self) -> i32 {
        self.tile_height
    }

    pub fn generate_preview(&self) -> bool {
        self.generate_preview
    }

    pub fn working_directory(&self) -> Option<&Path> {
        self.working_directory.as_deref()
    }

    pub fn tileset_root_dir(&self) -> Option<&Path> {
        self.tileset_root_dir.as_deref()
    }

    pub fn set_tile_size(&mut self, size: i32) {
        self.tile_width = size;
        self.tile_height = size;
    }

    pub fn set_working_directory<P: AsRef<Path>>(&mut self, working_directory: P) {
        self.working_directory = Some(working_directory.as_ref().to_path_buf());
    }

    pub fn set_tileset_root_dir<P: AsRef<Path>>(&mut self, tileset_root_dir: P) {
        self.tileset_root_dir = Some(tileset_root_dir.as_ref().to_path_buf());
    }

    pub fn set_generate_preview_html(&mut self, generate_preview: bool) {
        self.generate_preview = generate_preview;
    }

    pub fn write_html_preview(&self, html: &str) -> Result<(), TilingError> {
        if let Some(dir) = &self.tileset_root_dir {
            let preview = dir.join("preview.html");
            fs::write(preview, html)?;
        }
        Ok(())
    }

    pub fn convert(&mut self, image: &Path) -> Result<TileSetInfo, TilingError> {
        self.convert_to(
            image,
            self.tileset_root_dir.as_deref().unwrap_or(Path::new(".")),
        )
    }

    pub fn convert_to(&mut self, image: &Path, target: &Path) -> Result<TileSetInfo, TilingError> {
        if !target.exists() {
            fs::create_dir_all(target)?;
        }
        self.set_tileset_root_dir(target);

        let info = TileSetInfo::new(image, self.tile_width, self.tile_height, self.processor())?;
        self.convert_internal(image, info)
    }

    pub fn convert_internal(
        &mut self,
        image: &Path,
        info: TileSetInfo,
    ) -> Result<TileSetInfo, TilingError> {
        Ok(info)
    }
}
