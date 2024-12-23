use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use super::zoomify_tiler::{MAX_TILES_PER_GROUP, TILEGROUP};
use crate::validation_failed_exception::ValidationFailedError;
use crate::validator::Validator;

/// Validation class for the Zoomify tiling scheme.
pub struct ZoomifyValidator {
    /// File name of the descriptor file
    image_properties: &'static str,

    /// Number of expected TileGroup directories
    tile_groups: i32,

    /// Number of tiles in the last TileGroup
    tiles_in_last_group: i32,

    /// Tile size
    tile_size: i32,

    /// Number of tiles in the base layer in x-direction
    x_tiles: Vec<i32>,

    /// Number of tiles in the base layer in y-direction
    y_tiles: Vec<i32>,

    /// Number of zoomlevels in this tileset
    zoom_levels: i32,
}

impl ZoomifyValidator {
    pub fn new() -> Self {
        Self {
            image_properties: "ImageProperties.xml",
            tile_groups: 0,
            tiles_in_last_group: 0,
            tile_size: 0,
            x_tiles: Vec::new(),
            y_tiles: Vec::new(),
            zoom_levels: 0,
        }
    }

    fn parse_image_properties(&mut self, xml: &str) -> Result<(), ValidationFailedError> {
        let xml = xml.to_lowercase();

        // Helper function to extract value between quotes
        let extract_value = |attr: &str| -> Result<i32, ValidationFailedError> {
            let start = xml
                .find(&format!("{}=\"", attr))
                .ok_or_else(|| ValidationFailedError::new("Missing attribute"))?
                + attr.len()
                + 2;
            let end = xml[start..]
                .find('"')
                .ok_or_else(|| ValidationFailedError::new("Missing closing quote"))?;
            xml[start..start + end]
                .parse::<i32>()
                .map_err(|e| ValidationFailedError::new(format!("Invalid number: {}", e)))
        };

        let width = extract_value("width")?;
        let height = extract_value("height")?;
        let numtiles = extract_value("numtiles")?;
        self.tile_size = extract_value("tilesize")?;

        self.tile_groups = (numtiles as f64 / MAX_TILES_PER_GROUP as f64).ceil() as i32;
        self.tiles_in_last_group = numtiles % MAX_TILES_PER_GROUP;

        let x_base_tiles = (width as f64 / self.tile_size as f64).ceil() as i32;
        let y_base_tiles = (height as f64 / self.tile_size as f64).ceil() as i32;

        let max_tiles = x_base_tiles.max(y_base_tiles) as f64;
        self.zoom_levels = (max_tiles.log2()).ceil() as i32 + 1;

        self.x_tiles.clear();
        self.y_tiles.clear();

        let mut x = x_base_tiles as f64;
        let mut y = y_base_tiles as f64;

        for _ in 0..self.zoom_levels {
            self.x_tiles.push(x.ceil() as i32);
            self.y_tiles.push(y.ceil() as i32);
            x /= 2.0;
            y /= 2.0;
        }

        Ok(())
    }

    fn check_tile_directories(&self, tileset_dir: &Path) -> Result<(), ValidationFailedError> {
        let mut all_tiles: HashMap<i32, HashSet<String>> = HashMap::new();

        for entry in fs::read_dir(tileset_dir)? {
            let entry = entry?;
            let file_name = entry.file_name().to_string_lossy().into_owned();

            if file_name.contains(TILEGROUP) {
                let tile_group: i32 = file_name[TILEGROUP.len()..]
                    .parse()
                    .map_err(|_| ValidationFailedError::new("Invalid TileGroup number"))?;

                let tiles: HashSet<String> = fs::read_dir(&entry.path())?
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect();

                if tile_group < self.tile_groups - 2 {
                    if tiles.len() < MAX_TILES_PER_GROUP as usize {
                        return Err(ValidationFailedError::new(format!(
                            "Missing tiles in directory {} ({} instead of {})",
                            file_name,
                            tiles.len(),
                            MAX_TILES_PER_GROUP
                        )));
                    }
                } else if tiles.len() < self.tiles_in_last_group as usize {
                    return Err(ValidationFailedError::new(format!(
                        "Missing tiles in directory {} ({} instead of {})",
                        file_name,
                        tiles.len(),
                        self.tiles_in_last_group
                    )));
                }

                all_tiles.insert(tile_group, tiles);
            }
        }

        self.check_for_each_tile(&all_tiles)?;
        Ok(())
    }

    fn check_for_each_tile(
        &self,
        all_tiles: &HashMap<i32, HashSet<String>>,
    ) -> Result<(), ValidationFailedError> {
        let mut tile = 0;

        for zoom_level in (0..self.zoom_levels).rev() {
            for row in 0..self.y_tiles[zoom_level as usize] {
                for col in 0..self.x_tiles[zoom_level as usize] {
                    let tile_name =
                        format!("{}-{}-{}.jpg", self.zoom_levels - 1 - zoom_level, col, row);
                    let tile_group = tile / MAX_TILES_PER_GROUP;

                    if !all_tiles
                        .get(&tile_group)
                        .map_or(false, |tiles| tiles.contains(&tile_name))
                    {
                        return Err(ValidationFailedError::new(format!(
                            "Missing tile: {}",
                            tile_name
                        )));
                    }
                    tile += 1;
                }
            }
        }

        Ok(())
    }
}

impl Validator for ZoomifyValidator {
    fn is_tileset_dir<P: AsRef<Path>>(&self, dir: P) -> bool {
        if !dir.as_ref().is_dir() {
            return false;
        }

        fs::read_dir(dir.as_ref()).ok().map_or(false, |entries| {
            entries
                .filter_map(|e| e.ok())
                .any(|e| e.file_name() == self.image_properties)
        })
    }

    fn validate<P: AsRef<Path>>(&self, dir: P) -> Result<(), ValidationFailedError> {
        let dir = dir.as_ref();
        if !dir.is_dir() {
            return Err(ValidationFailedError::new("Not a zoomify tileset"));
        }

        let properties_file = dir.join(self.image_properties);
        if !properties_file.exists() {
            return Err(ValidationFailedError::new(
                "Not a Zoomify tileset - missing ImageProperties.xml",
            ));
        }

        let file = File::open(properties_file)?;
        let reader = BufReader::new(file);
        let xml: String = reader.lines().filter_map(|line| line.ok()).collect();

        self.parse_image_properties(&xml)?;
        self.check_tile_directories(dir)?;

        Ok(())
    }
}
