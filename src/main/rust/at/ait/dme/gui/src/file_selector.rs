use eframe::egui;
use rfd::FileDialog;
use std::path::PathBuf;

pub struct FileSelector {
    path: Option<PathBuf>,
    label: String,
    filter_name: String,
    extensions: Vec<String>,
}

impl FileSelector {
    pub fn new(label: &str, filter_name: &str, extensions: Vec<&str>) -> Self {
        Self {
            path: None,
            label: label.to_string(),
            filter_name: filter_name.to_string(),
            extensions: extensions.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            if ui.button("Browse...").clicked() {
                if let Some(path) = FileDialog::new()
                    .add_filter(
                        &self.filter_name,
                        &self
                            .extensions
                            .iter()
                            .map(|s| s.as_str())
                            .collect::<Vec<_>>(),
                    )
                    .pick_file()
                {
                    self.path = Some(path);
                    changed = true;
                }
            }

            ui.label(&self.label);
            if let Some(path) = &self.path {
                ui.label(path.to_string_lossy().to_string());
            } else {
                ui.label("No file selected");
            }
        });
        changed
    }

    pub fn path(&self) -> Option<&PathBuf> {
        self.path.as_ref()
    }

    pub fn clear(&mut self) {
        self.path = None;
    }
}
