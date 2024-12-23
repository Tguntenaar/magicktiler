use eframe::egui;
use log::{error, info};
use std::path::PathBuf;

use magicktiler::{gmaps::GoogleMapsTiler, tms::TMSTiler, zoomify::ZoomifyTiler, MagickTiler};

use crate::file_selector::FileSelector;
use crate::radio_button_group::RadioButtonGroup;

pub struct MagickTilerApp {
    input_selector: FileSelector,
    output_selector: FileSelector,
    tiling_scheme: RadioButtonGroup,
    tile_size: RadioButtonGroup,
    generate_preview: bool,
    processing: bool,
    status: String,
}

impl MagickTilerApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self {
            input_selector: FileSelector::new(
                "Input Image",
                "Image files",
                vec!["jpg", "jpeg", "png", "tif", "tiff"],
            ),
            output_selector: FileSelector::new("Output Directory", "All files", vec!["*"]),
            tiling_scheme: RadioButtonGroup::new(
                "Tiling Scheme",
                vec!["Zoomify", "Google Maps", "TMS"],
            ),
            tile_size: RadioButtonGroup::new("Tile Size", vec!["256x256", "512x512"]),
            generate_preview: true,
            processing: false,
            status: String::new(),
        }
    }

    fn process_image(&mut self) {
        self.processing = true;
        self.status = "Processing...".to_string();

        let input_path = match self.input_selector.path() {
            Some(path) => path.clone(),
            None => {
                self.status = "No input file selected".to_string();
                self.processing = false;
                return;
            }
        };

        let output_path = match self.output_selector.path() {
            Some(path) => path.clone(),
            None => {
                self.status = "No output directory selected".to_string();
                self.processing = false;
                return;
            }
        };

        let result = match self.tiling_scheme.selected() {
            0 => self.process_with_tiler(ZoomifyTiler::new(), &input_path, &output_path),
            1 => self.process_with_tiler(GoogleMapsTiler::new(), &input_path, &output_path),
            2 => self.process_with_tiler(TMSTiler::new(), &input_path, &output_path),
            _ => unreachable!(),
        };

        match result {
            Ok(_) => {
                self.status = "Processing complete".to_string();
                info!("Processing complete");
            }
            Err(e) => {
                self.status = format!("Error: {}", e);
                error!("Processing failed: {}", e);
            }
        }

        self.processing = false;
    }

    fn process_with_tiler<T: MagickTiler>(
        &self,
        mut tiler: T,
        input: &PathBuf,
        output: &PathBuf,
    ) -> Result<(), Box<dyn std::error::Error>> {
        tiler.set_tile_size(if self.tile_size.selected() == 0 {
            256
        } else {
            512
        });
        tiler.set_generate_preview(self.generate_preview);
        tiler.convert_to(input, output)?;
        Ok(())
    }
}

impl eframe::App for MagickTilerApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("MagickTiler");
            ui.add_space(20.0);

            self.input_selector.show(ui);
            ui.add_space(10.0);
            self.output_selector.show(ui);
            ui.add_space(20.0);

            self.tiling_scheme.show(ui);
            ui.add_space(20.0);
            self.tile_size.show(ui);
            ui.add_space(20.0);

            ui.checkbox(&mut self.generate_preview, "Generate Preview");
            ui.add_space(20.0);

            if !self.processing {
                if ui.button("Process").clicked() {
                    self.process_image();
                }
            } else {
                ui.spinner();
            }

            if !self.status.is_empty() {
                ui.add_space(20.0);
                ui.label(&self.status);
            }
        });
    }
}
