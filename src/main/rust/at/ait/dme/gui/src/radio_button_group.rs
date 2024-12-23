use eframe::egui;

pub struct RadioButtonGroup {
    selected: usize,
    options: Vec<String>,
    label: String,
}

impl RadioButtonGroup {
    pub fn new(label: &str, options: Vec<&str>) -> Self {
        Self {
            selected: 0,
            options: options.iter().map(|s| s.to_string()).collect(),
            label: label.to_string(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.vertical(|ui| {
            ui.label(&self.label);
            for (idx, option) in self.options.iter().enumerate() {
                if ui.radio_value(&mut self.selected, idx, option).changed() {
                    changed = true;
                }
            }
        });
        changed
    }

    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn selected_text(&self) -> &str {
        &self.options[self.selected]
    }
}
