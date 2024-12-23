use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};

use log::error;

use super::google_maps_tiler::METADATA_FILE;
use crate::tile_set_info::TileSetInfo;
use crate::validation_failed_exception::ValidationFailedError;
use crate::validator::Validator;

/// Validator for the Google Maps tiling scheme.
pub struct GoogleMapsValidator;

impl GoogleMapsValidator {
    pub fn new() -> Self {
        Self
    }

    fn read_metadata(&self, dir: &Path) -> Result<TileSetInfo, ValidationFailedError> {
        let metadata_path = dir.join(METADATA_FILE);
        let mut metadata = String::new();
        File::open(&metadata_path)
            .map_err(|_| ValidationFailedError::new("Metadata file not found!"))?
            .read_to_string(&mut metadata)
            .map_err(|e| {
                error!("Could not read metadata file: {}", e);
                ValidationFailedError::new("Failed to read metadata")
            })?;

        serde_json::from_str(&metadata).map_err(|e| {
            error!("Could not parse metadata: {}", e);
            ValidationFailedError::new("Failed to parse metadata")
        })
    }
}

impl Validator for GoogleMapsValidator {
    fn is_tileset_dir<P: AsRef<Path>>(&self, dir: P) -> bool {
        if !dir.as_ref().is_dir() {
            return false;
        }

        fs::read_dir(dir.as_ref()).ok().map_or(false, |entries| {
            entries
                .filter_map(|e| e.ok())
                .any(|e| e.file_name() == METADATA_FILE)
        })
    }

    fn validate<P: AsRef<Path>>(&self, dir: P) -> Result<(), ValidationFailedError> {
        let dir = dir.as_ref();
        if !self.is_tileset_dir(dir) {
            return Err(ValidationFailedError::new(
                "Not a MagickTiler Google Maps tileset, validation cannot be continued.",
            ));
        }

        let info = self.read_metadata(dir)?;
        let mut files_verified = 0;

        for z in 0..info.zoom_levels() {
            for x in 0..info.number_of_x_tiles(info.zoom_levels() - 1 - z) {
                for y in 0..info.number_of_y_tiles(info.zoom_levels() - 1 - z) {
                    let tile = format!("{}_{}_{}_{}", z, x, y, info.tile_format().extension());
                    if !dir.join(&tile).exists() {
                        return Err(ValidationFailedError::new(format!(
                            "Files missing for zoom level {}",
                            z
                        )));
                    }
                    files_verified += 1;
                }
            }
        }

        if files_verified != info.total_number_of_tiles() {
            return Err(ValidationFailedError::new(
                "Not enough files generated for Tileset!",
            ));
        }

        Ok(())
    }
}
