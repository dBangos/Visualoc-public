use egui::{ColorImage, Grid};
use uuid::Uuid;

use crate::{CommandToServer, Container, ContainerScreen, ModalType, UIPages, Visualoc};

impl Visualoc {
    pub fn location_grid_screen(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if self.rearrange_locations {
                ui.add_space(ui.available_width() / 2.0 - 95.0);
                if Visualoc::ok_button(ui).clicked() {
                    self.ordered_locations_vec = self.new_ordered_locations_vec.clone();
                    self.rearrange_locations = false;
                }
                if Visualoc::cancel_button(ui).clicked() {
                    self.sort_locations(&self.ordered_locations_vec.clone());
                    self.rearrange_locations = false;
                }
            } else {
                ui.add_space(ui.available_width() / 2.0 - 150.0);
                if Visualoc::add_button(ui, "Add Location").clicked() {
                    self.selected_location = Container::default();
                    self.selected_location.master = "Source".to_string();
                    self.modal_vars.modal_type = ModalType::AddLocation;
                    self.async_tasks_to_send.push(CommandToServer::AddContainer(
                        Uuid::new_v4().to_string(),
                        self.selected_location.clone(),
                    ));
                    self.ordered_locations_vec
                        .push(self.selected_location.id.clone());
                }
                if ui.button("🔀 Rearrange Locations").clicked() {
                    //Copy the data over ot the new vec so the user can start modifying it
                    self.new_ordered_locations_vec = self.ordered_locations_vec.clone();
                    self.rearrange_locations = true;
                }
            }
        });
        ui.separator();
        //Area containing the location images
        egui::ScrollArea::vertical().show(ui, |ui| {
            let image_size = ui.available_size();
            Grid::new("LocationGrid")
                .min_col_width(image_size.x / 4.0)
                .min_row_height(image_size.y / 3.0)
                .show(ui, |ui| {
                    for (index, location) in self.container_vec.clone().iter().enumerate() {
                        if index % 4 == 0 && index > 0 {
                            ui.end_row();
                        }
                        ui.vertical_centered(|ui| {
                            let image_name = location.id.clone() + "." + &location.image_type;
                            if self.loaded_images.contains_key(&image_name) {
                                if let Some((texture, _)) = self.loaded_images[&image_name].clone()
                                {
                                    let current_image = egui::Image::new(&texture)
                                        .maintain_aspect_ratio(true)
                                        .max_height(image_size.y / 4.0)
                                        .max_width(image_size.x / 3.0)
                                        .sense(egui::Sense::click());
                                    if ui.add(current_image).clicked() {
                                        self.selected_location = location.clone();
                                        self.item_vec = Vec::new();
                                        self.container_screen = ContainerScreen::None;
                                        self.selected_container = Container::default();
                                        //Remove the small pic so the large can be loaded
                                        self.loaded_images.remove(&image_name);
                                        self.prepare_page(UIPages::LocationContainers);
                                    }
                                }
                            } else {
                                self.loaded_images.insert(image_name.to_owned(), None);
                                self.async_tasks_to_send
                                    .push(CommandToServer::GetImageFromServer(
                                        Uuid::new_v4().to_string(),
                                        location.id.clone(),
                                        location.image_type.clone(),
                                        crate::database::data_helpers::ImageSize::Medium,
                                        ColorImage::default(),
                                    ));
                            }
                            if ui.label(&location.name).clicked() {
                                self.selected_location = location.clone();
                                self.item_vec = Vec::new();
                                self.container_screen = ContainerScreen::None;
                                self.selected_container = Container::default();
                                self.loaded_images.remove(&image_name);
                                self.prepare_page(UIPages::LocationContainers);
                            }
                            if self.rearrange_locations {
                                //If for some reason the order size doesn't match the location count, reset the order
                                if self.new_ordered_locations_vec.len() != self.container_vec.len()
                                {
                                    self.new_ordered_locations_vec =
                                        self.container_vec.iter().map(|x| x.id.clone()).collect();
                                }
                                ui.horizontal(|ui| {
                                    ui.add_space(ui.available_width() / 2.0 - 80.0);
                                    if ui.button("Left ⬅").clicked() && index > 0 {
                                        self.new_ordered_locations_vec.swap(index, index - 1);
                                        self.sort_locations(
                                            &self.new_ordered_locations_vec.clone(),
                                        );
                                    }
                                    if ui.button("➡Right").clicked()
                                        && index < self.new_ordered_locations_vec.len() - 1
                                    {
                                        self.new_ordered_locations_vec.swap(index, index + 1);
                                        self.sort_locations(
                                            &self.new_ordered_locations_vec.clone(),
                                        );
                                    }
                                });
                            }
                            ui.separator();
                        });
                    }
                });
        });
    }
}
