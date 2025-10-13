use uuid::Uuid;

use crate::{Backup, CommandToServer, DataType, FieldModal, ModalType, UIPages, Visualoc};

impl Visualoc {
    pub fn top_row(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("🏠Home").clicked() {
                self.home_page.column_search = ("Name".to_string(), DataType::String);
                self.search_string = String::new();
                self.home_page.previous_search = String::new();
                self.async_tasks_to_send.push(CommandToServer::SearchItems(
                    Uuid::new_v4().to_string(),
                    self.search_string.clone(),
                    self.home_page.column_search.0.clone(),
                    Vec::new(),
                ));
                self.prepare_page(UIPages::Home);
            }
            if ui.button("⛃ Locations").clicked() {
                self.prepare_page(UIPages::LocationGrid);
            }
            if Visualoc::add_button(ui, "Add/Delete Item Fields").clicked() {
                self.modal_vars.field_modal = FieldModal::Start;
            }
            if ui.button("📊 Statistics").clicked() {
                self.prepare_page(UIPages::Statistics);
            }
            if ui.button("💾 Backup").clicked() {
                self.backup = Backup::default();
                self.modal_vars.modal_type = ModalType::Backup;
            }

            ui.add_space(ui.available_width() - 230.0);
            if ui.button("👤 Account").clicked() {
                self.prepare_page(UIPages::Account);
            }
            if ui.button("⛭ Settings").clicked() {
                self.modal_vars.modal_type = ModalType::Settings;
                self.temp_settings = self.settings.clone();
            }
        });
    }
}
