use std::collections::HashMap;
use std::io;
use std::path::{Path, PathBuf};

use crate::image::ImageProcessingSystem;
use crate::image::ImageProcessor;

/// To speed up the MagickTiler tiling process, images are (for most tiling schemes)
/// first split into a sequence of 'stripes'. Depending on the tiling scheme, striping
/// is done either vertically or horizontally. This struct is a utility for handling
/// and manipulating image stripes.
#[derive(Debug, Clone)]
pub struct Stripe {
    /// The stripe image file
    file: PathBuf,

    /// The width of this stripe in pixels
    width: i32,

    /// The height of this stripe in pixels
    height: i32,

    /// This stripe's orientation
    orientation: Orientation,
}

/// Possible stripe orientations
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Orientation {
    Horizontal,
    Vertical,
}

impl Stripe {
    const DIFFERENT_ORIENTATION_ERROR: &'static str =
        "Cannot merge. Stripes have different orientation";

    pub fn new<P: AsRef<Path>>(file: P, width: i32, height: i32, orientation: Orientation) -> Self {
        Self {
            file: file.as_ref().to_path_buf(),
            width,
            height,
            orientation,
        }
    }

    pub fn image_file(&self) -> &Path {
        &self.file
    }

    pub fn width(&self) -> i32 {
        self.width
    }

    pub fn height(&self) -> i32 {
        self.height
    }

    pub fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Merges this stripe with another one into a single stripe, scaled according to
    /// the resolution of the next pyramid zoom layer. (I.e. the two original stripes
    /// will be joined next to each other, and the resulting image will be down-scaled
    /// by 50%).
    pub fn merge<P: AsRef<Path>>(
        &self,
        stripe: &Stripe,
        target_file: P,
        system: ImageProcessingSystem,
    ) -> io::Result<Stripe> {
        self.merge_with_canvas(stripe, None, -1, -1, None, target_file, system)
    }

    /// Merges this stripe with another one into a single stripe, scaled according to
    /// the resolution of the next pyramid zoom layer. (I.e. the two original stripes
    /// will be joined next to each other, and the resulting image will be down-scaled
    /// by 50%).
    /// This method allows to create a background color buffer around the stripe, in case
    /// the employed tiling scheme mandates certain image resolution constraints (e.g.
    /// width/height must be integer multiples of the tile-size).
    pub fn merge_with_canvas<P: AsRef<Path>>(
        &self,
        stripe: &Stripe,
        gravity: Option<&str>,
        x_extent: i32,
        y_extent: i32,
        background_color: Option<&str>,
        target_file: P,
        system: ImageProcessingSystem,
    ) -> io::Result<Stripe> {
        if stripe.orientation != self.orientation {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                Self::DIFFERENT_ORIENTATION_ERROR,
            ));
        }

        let mut srcs = vec![
            self.file.to_string_lossy().into_owned(),
            stripe.file.to_string_lossy().into_owned(),
        ];

        let (x_tiles, y_tiles) = match self.orientation {
            Orientation::Horizontal => (1, 2),
            Orientation::Vertical => (2, 1),
        };

        let processor = ImageProcessor::new(system);

        if x_extent > -1 && y_extent > -1 {
            let w = x_extent;
            let h = y_extent;
            let w = w / 2;

            if let Some(gravity) = gravity {
                if let Some(bg_color) = background_color {
                    processor.montage(
                        &srcs,
                        &target_file,
                        x_tiles,
                        y_tiles,
                        w,
                        h,
                        Some(bg_color.to_string()),
                        Some(gravity.to_string()),
                    )?;
                }
            }

            Ok(Stripe::new(target_file, w, h, self.orientation))
        } else {
            let w = match self.orientation {
                Orientation::Horizontal => self.width / 2,
                Orientation::Vertical => (self.width + stripe.width) / 2,
            };
            let h = match self.orientation {
                Orientation::Horizontal => (self.height + stripe.height) / 4,
                Orientation::Vertical => self.height / 2,
            };

            let mut raw_args = HashMap::new();
            raw_args.insert("-geometry".to_string(), "+0+0".to_string());
            raw_args.insert("-resize".to_string(), "50%x50%".to_string());

            processor.montage(&srcs, &target_file, x_tiles, y_tiles, Some(raw_args))?;

            Ok(Stripe::new(target_file, w, h, self.orientation))
        }
    }

    /// Shrinks this stripe 50% to the resolution of the next zoom level.
    pub fn shrink<P: AsRef<Path>>(
        &self,
        target_file: P,
        system: ImageProcessingSystem,
    ) -> io::Result<Stripe> {
        self.shrink_with_canvas(None, -1, -1, None, target_file, system)
    }

    /// Shrinks this stripe 50% to the resolution of the next zoom level.
    /// This method allows to create a background color buffer around the stripe, in case
    /// the employed tiling scheme mandates certain image resolution constraints (e.g.
    /// width/height must be integer multiples of the tile-size).
    pub fn shrink_with_canvas<P: AsRef<Path>>(
        &self,
        gravity: Option<&str>,
        x_extent: i32,
        y_extent: i32,
        background_color: Option<&str>,
        target_file: P,
        system: ImageProcessingSystem,
    ) -> io::Result<Stripe> {
        let processor = ImageProcessor::new(system);

        if x_extent > -1 && y_extent > -1 {
            let mut srcs = vec![
                self.file.to_string_lossy().into_owned(),
                "null:".to_string(),
            ];

            let (x_tiles, y_tiles) = match self.orientation {
                Orientation::Horizontal => (1, 2),
                Orientation::Vertical => (2, 1),
            };

            if let Some(gravity) = gravity {
                if let Some(bg_color) = background_color {
                    processor.montage(
                        &srcs,
                        &target_file,
                        x_tiles,
                        y_tiles,
                        x_extent / 2,
                        y_extent,
                        Some(bg_color.to_string()),
                        Some(gravity.to_string()),
                    )?;
                }
            }

            Ok(Stripe::new(
                target_file,
                x_extent,
                y_extent,
                self.orientation,
            ))
        } else {
            let mut raw_args = HashMap::new();
            raw_args.insert("-scale".to_string(), "50%x50%".to_string());

            processor.convert(&self.file, &target_file, Some(raw_args))?;

            Ok(Stripe::new(
                target_file,
                self.width / 2,
                self.height / 2,
                self.orientation,
            ))
        }
    }

    /// Removes this stripe's image file from the file system.
    /// (Note that stripes are normally used as temporary files only!)
    pub fn delete(&self) -> io::Result<()> {
        std::fs::remove_file(&self.file).map_err(|e| {
            io::Error::new(
                e.kind(),
                format!("Could not delete file: {}", self.file.display()),
            )
        })
    }
}
