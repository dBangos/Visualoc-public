use egui::Ui;

use crate::Visualoc;

impl Visualoc {
    pub fn account_page(&mut self, ui: &mut Ui) {
        ui.label("Email:");
        ui.label("Username:");
        ui.label("Password:");
        if ui.button("Change Password").clicked() {
            //
        }
        if ui.button("Logout").clicked() {
            *self = Visualoc::default();
        }
    }
}
