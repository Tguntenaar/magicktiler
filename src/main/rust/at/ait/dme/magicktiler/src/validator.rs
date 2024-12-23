use crate::validation_failed_exception::ValidationFailedError;
use std::path::Path;

/// Interface for file and tiling scheme validators.
pub trait Validator {
    /// Returns true if the directory is (potentially) a tileset
    /// directory, e.g. because it contains a tileset descriptor file.
    ///
    /// # Arguments
    /// * `dir` - The directory to check
    fn is_tileset_dir<P: AsRef<Path>>(&self, dir: P) -> bool;

    /// Validate a tileset
    ///
    /// # Arguments
    /// * `dir` - The tileset directory to validate
    ///
    /// # Errors
    /// Returns a ValidationFailedError if the tileset fails validation
    fn validate<P: AsRef<Path>>(&self, dir: P) -> Result<(), ValidationFailedError>;
}
