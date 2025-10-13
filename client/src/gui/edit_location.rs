use egui::{
    Align, Color32, ColorImage, ComboBox, Context, DragValue, Label, Layout, RichText, ScrollArea,
    Slider, TextEdit,
};
use egui_extras::Column;
use uuid::Uuid;

use crate::{
    CommandToServer, ContainedItem, Container, ContainerScreen, DataType, ModalType, UIPages,
    Visualoc, WaitingFunctionKind,
};

impl Visualoc {
    pub fn initial_location_screen(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("Location Actions");
        });
        ui.with_layout(
            Layout::left_to_right(Align::LEFT).with_main_wrap(true),
            |ui| {
                if Visualoc::add_button(ui, "Add Container").clicked() {
                    self.selected_container = Container::default();
                    self.selected_container.master = self.selected_location.id.clone();
                    self.container_screen = ContainerScreen::AddingContainer;
                }
                if ui.button("✏ Edit Location").clicked() {
                    self.container_screen = ContainerScreen::EditingLocation;
                }
                if Visualoc::delete_button(ui, "Delete Location").clicked() {
                    self.modal_vars.modal_type = ModalType::DeleteLocation;
                }
            },
        );
    }

    pub fn editing_location_screen(&mut self, ui: &mut egui::Ui) {
        ui.label("Edit Location");
        ui.horizontal(|ui| {
            ui.label("Name ");
            ui.add(
                egui::TextEdit::singleline(&mut self.selected_location.name)
                    .hint_text("Location Name"),
            );
        });
        ui.add_space(5.0);
        let mut temp_string = "➕ Add Image";
        if self.selected_location.image_type != String::default() {
            temp_string = "Change Image"
        }
        if ui.button(temp_string).clicked() {
            self.loaded_images.remove(
                &(self.selected_location.id.to_string() + "." + &self.selected_location.image_type),
            );
            self.async_tasks_to_send.push(CommandToServer::AddImage(
                Uuid::new_v4().to_string(),
                self.selected_location.id.clone(),
                String::new(),
            ));
            self.redraw_canvas_image = true;
        }
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            if Visualoc::ok_button(ui).clicked() {
                self.async_tasks_to_send
                    .push(CommandToServer::UpdateContainer(
                        Uuid::new_v4().to_string(),
                        self.selected_location.clone(),
                    ));
                self.container_screen = ContainerScreen::None;
            }
            if Visualoc::cancel_button(ui).clicked() {
                //Delete new image
                self.loaded_images.remove(
                    &(self.selected_location.id.to_string()
                        + "."
                        + &self.selected_location.image_type),
                );
                self.redraw_canvas_image = true;
                self.container_screen = ContainerScreen::None;
            }
        });
    }

    fn show_all_containers(&mut self, ui: &mut egui::Ui) {
        for container in &self.container_vec {
            let button = egui::Button::new(&container.name);
            let response = ui.add(button);
            if response.clicked() {
                self.selected_container = container.clone();
                self.async_tasks_to_send
                    .push(CommandToServer::GetMultipleItems(
                        Uuid::new_v4().to_string(),
                        self.selected_container.contained_items.clone(),
                        Vec::new(),
                    ));

                self.search_string = "".into();
                self.item_page_search_vec = self.item_vec.clone();
                self.container_screen = ContainerScreen::SelectedContainer;
            }
            if self.selected_container.id == container.id {
                response.highlight();
            }
        }
    }

    fn item_area(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered_justified(|ui| {
            ui.add(TextEdit::singleline(&mut self.search_string).hint_text("Search Items"));
        });
        self.item_page_search_vec = self
            .item_vec
            .clone()
            .into_iter()
            .filter(|x| x.name.contains(&self.search_string))
            .collect();
        ui.add_space(4.0);
        ScrollArea::vertical()
            .stick_to_right(true)
            .auto_shrink(false)
            .show(ui, |ui| {
                for item in &self.item_page_search_vec {
                    if ui.button(&item.name).clicked() {
                        self.selected_item = item.clone();
                        self.container_screen = ContainerScreen::SelectedItem;
                    }
                }
            });
    }

    pub fn container_selected_screen(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading("Container Options");
        });
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            if Visualoc::add_button(ui, "Add Item").clicked() {
                self.selected_item = ContainedItem::default();
                self.dynamic_fields_initialization();
                self.async_tasks_to_send.push(CommandToServer::InsertItem(
                    Uuid::new_v4().to_string(),
                    self.selected_item.clone(),
                    self.selected_container.id.clone(),
                    self.item_field_types.clone(),
                ));
                self.container_screen = ContainerScreen::AddingItem;
            }
            if Visualoc::add_button(ui, "Add Existing Item").clicked() {
                let cmd_id = Uuid::new_v4().to_string();
                self.async_tasks_to_send
                    .push(CommandToServer::GetAllItemIdsNotInContainer(
                        cmd_id.clone(),
                        Vec::new(),
                    ));
                self.functions_waiting_data.push(crate::WaitingFunction {
                    id: cmd_id,
                    kind: WaitingFunctionKind::AddExistingItemClicked1,
                });
            }
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            if ui.button("✏ Edit Container").clicked() {
                self.container_screen = ContainerScreen::EditingContainer;
            }
            if Visualoc::delete_button(ui, "Delete Container").clicked() {
                self.modal_vars.modal_type = ModalType::DeleteContainer;
            }
        });
        ui.add_space(4.0);
        if ui.button("⮪ Back to Location").clicked() {
            self.container_screen = ContainerScreen::None;
        }
        ui.separator();
        ui.add_space(5.0);
        ui.vertical_centered(|ui| {
            ui.heading("Items");
        });
        self.item_area(ui);
    }

    pub fn add_edit_container_screen(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            if self.container_screen == ContainerScreen::AddingContainer {
                ui.label("Adding Container");
            } else if self.container_screen == ContainerScreen::EditingContainer {
                ui.label("Editing Container");
            }
        });
        ui.add_space(5.0);
        ui.label("1. Click and drag on the main image to create a container");
        ui.separator();
        ui.add_space(5.0);
        ui.label("2. Container details");
        // ui.vertical_centered(|ui| {
        //     if self.selected_container.image_type != String::default() {
        //         // let image_path = get_image_path(
        //         //     &self.host,
        //         //     &self.selected_container.id,
        //         //     &self.selected_container.image_type,
        //         // );
        //         // ui.add(
        //         //     egui::Image::new(image_path)
        //         //         .fit_to_original_size(2.0)
        //         //         .max_height(70.0)
        //         //         .sense(egui::Sense::click()),
        //         // );
        //     }
        // });
        // ui.add_space(5.0);
        // if Visualoc::add_button(ui, "Add Image").clicked() {
        //     self.async_tasks_to_send.push(CommandToServer::AddImage(
        //         Uuid::new_v4().to_string(),
        //         self.selected_container.id.clone(),
        //         String::new(),
        //     ));
        // }
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            ui.label("Name ");
            ui.add(
                egui::TextEdit::singleline(&mut self.selected_container.name)
                    .hint_text("Container Name"),
            );
        });
        ui.add_space(5.0);
        ui.horizontal(|ui| {
            if Visualoc::ok_button(ui).clicked() {
                if self.container_screen == ContainerScreen::AddingContainer {
                    self.async_tasks_to_send.push(CommandToServer::AddContainer(
                        Uuid::new_v4().to_string(),
                        self.selected_container.clone(),
                    ));
                } else if self.container_screen == ContainerScreen::EditingContainer {
                    self.async_tasks_to_send
                        .push(CommandToServer::UpdateContainer(
                            Uuid::new_v4().to_string(),
                            self.selected_container.clone(),
                        ));
                }
                self.async_tasks_to_send.push(CommandToServer::GetAllSlaves(
                    Uuid::new_v4().to_string(),
                    self.selected_location.id.clone(),
                    Vec::new(),
                ));
                self.container_screen = ContainerScreen::None;
            }
            if Visualoc::cancel_button(ui).clicked() {
                self.container_screen = ContainerScreen::None;
            }
        });
    }

    pub fn edit_or_label(
        &mut self,
        ui: &mut egui::Ui,
        datatype: DataType,
        column_name: String,
        index: usize,
        ctx: &Context,
    ) {
        match datatype {
            DataType::Float => {
                if self.container_screen == ContainerScreen::EditingItem
                    || self.container_screen == ContainerScreen::AddingItem
                {
                    ui.add(DragValue::new(&mut self.selected_item.float_vars[index]));
                } else {
                    ui.horizontal(|ui| {
                        let clicked = Visualoc::interactive_label(
                            ui,
                            ctx,
                            self.settings.light_mode,
                            true,
                            &self.selected_item.float_vars[index].to_string(),
                        );
                        if clicked {
                            self.prepare_page(UIPages::Home);
                            self.home_page.column_search = (column_name, DataType::String);
                            self.search_string = self.selected_item.float_vars[index].to_string();
                        }
                        Visualoc::copy_button(
                            ui,
                            self.settings.light_mode,
                            self.selected_item.float_vars[index].to_string(),
                            ctx,
                        )
                    });
                }
            }
            DataType::Integer => {
                if self.container_screen == ContainerScreen::EditingItem
                    || self.container_screen == ContainerScreen::AddingItem
                {
                    ui.add(DragValue::new(&mut self.selected_item.int_vars[index]));
                } else {
                    ui.horizontal(|ui| {
                        let clicked = Visualoc::interactive_label(
                            ui,
                            ctx,
                            self.settings.light_mode,
                            false,
                            &self.selected_item.int_vars[index].to_string(),
                        );
                        if clicked {
                            self.prepare_page(UIPages::Home);
                            self.home_page.column_search = (column_name, DataType::String);
                            self.search_string = self.selected_item.int_vars[index].to_string();
                        }
                        Visualoc::copy_button(
                            ui,
                            self.settings.light_mode,
                            self.selected_item.int_vars[index].to_string(),
                            ctx,
                        )
                    });
                }
            }
            DataType::String => {
                if self.container_screen == ContainerScreen::EditingItem
                    || self.container_screen == ContainerScreen::AddingItem
                {
                    ui.vertical_centered_justified(|ui| {
                        ui.add(egui::TextEdit::singleline(
                            &mut self.selected_item.string_vars[index],
                        ));
                    });
                } else {
                    ui.horizontal(|ui| {
                        let clicked = Visualoc::interactive_label(
                            ui,
                            ctx,
                            self.settings.light_mode,
                            true,
                            &self.selected_item.string_vars[index],
                        );
                        if clicked {
                            self.prepare_page(UIPages::Home);
                            self.home_page.column_search = (column_name, DataType::String);
                            self.search_string = self.selected_item.string_vars[index].clone();
                        }
                        Visualoc::copy_button(
                            ui,
                            self.settings.light_mode,
                            self.selected_item.string_vars[index].clone(),
                            ctx,
                        )
                    });
                }
            }
            DataType::Percentage => {
                if self.container_screen == ContainerScreen::EditingItem
                    || self.container_screen == ContainerScreen::AddingItem
                {
                    ui.add(
                        Slider::new(&mut self.selected_item.float_vars[index], 0.0..=100.0)
                            .step_by(0.1)
                            .min_decimals(1),
                    );
                } else {
                    ui.horizontal(|ui| {
                        let clicked = Visualoc::interactive_label(
                            ui,
                            ctx,
                            self.settings.light_mode,
                            false,
                            &(self.selected_item.float_vars[index].to_string() + "%"),
                        );
                        if clicked {
                            self.prepare_page(UIPages::Home);
                            self.home_page.column_search = (column_name, DataType::Percentage);
                            self.search_string = self.selected_item.float_vars[index].to_string();
                        }
                        Visualoc::copy_button(
                            ui,
                            self.settings.light_mode,
                            self.selected_item.float_vars[index].to_string(),
                            ctx,
                        )
                    });
                }
            }
            DataType::Bool => {
                let mut val: bool = if self.selected_item.int_vars[index] == 0 {
                    false
                } else {
                    true
                };
                if self.container_screen == ContainerScreen::EditingItem
                    || self.container_screen == ContainerScreen::AddingItem
                {
                    ui.checkbox(&mut val, "");
                    if val == true {
                        self.selected_item.int_vars[index] = 1;
                    } else {
                        self.selected_item.int_vars[index] = 0;
                    }
                } else {
                    let symbol = if self.selected_item.int_vars[index] == 0 {
                        "✖"
                    } else {
                        "✅"
                    };
                    ui.horizontal(|ui| {
                        let clicked = Visualoc::interactive_label(
                            ui,
                            ctx,
                            self.settings.light_mode,
                            true,
                            &symbol.to_string(),
                        );
                        if clicked {
                            self.prepare_page(UIPages::Home);
                            self.home_page.column_search = (column_name, DataType::Bool);
                            self.search_string = self.selected_item.int_vars[index].to_string();
                        }
                        Visualoc::copy_button(
                            ui,
                            self.settings.light_mode,
                            self.selected_item.int_vars[index].to_string(),
                            ctx,
                        )
                    });
                }
            }
            DataType::Text => {
                if self.container_screen == ContainerScreen::EditingItem
                    || self.container_screen == ContainerScreen::AddingItem
                {
                    ui.vertical_centered_justified(|ui| {
                        ui.add(egui::TextEdit::multiline(
                            &mut self.selected_item.string_vars[index],
                        ));
                    });
                } else {
                    ui.add(
                        Label::new(RichText::new(&self.selected_item.string_vars[index])).wrap(),
                    );
                }
            }
            DataType::Gallery => {
                ui.label("todo");
            }
            DataType::List(string_vec) => {
                if self.container_screen == ContainerScreen::EditingItem
                    || self.container_screen == ContainerScreen::AddingItem
                {
                    ComboBox::from_label("")
                        .selected_text(self.selected_item.string_vars[index].clone())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut self.selected_item.string_vars[index],
                                String::new(),
                                String::new(),
                            );
                            for field in string_vec {
                                ui.selectable_value(
                                    &mut self.selected_item.string_vars[index],
                                    field.clone(),
                                    field,
                                );
                            }
                        });
                } else {
                    ui.horizontal(|ui| {
                        let clicked = Visualoc::interactive_label(
                            ui,
                            ctx,
                            self.settings.light_mode,
                            false,
                            &self.selected_item.string_vars[index],
                        );
                        if clicked {
                            self.prepare_page(UIPages::Home);
                            self.home_page.column_search =
                                (column_name, DataType::List(string_vec));
                            self.search_string = self.selected_item.string_vars[index].to_string();
                        }
                        Visualoc::copy_button(
                            ui,
                            self.settings.light_mode,
                            self.selected_item.string_vars[index].to_string(),
                            ctx,
                        )
                    });
                }
            }
        }
    }

    pub fn show_item_fields(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        ui.vertical_centered(|ui| {
            ui.horizontal(|ui| {
                if self.selected_item.image_type != String::new() {
                    let image_name =
                        &(self.selected_item.id.clone() + "." + &self.selected_item.image_type);
                    if self.loaded_images.contains_key(image_name) {
                        if let Some((texture, _)) = self.loaded_images[image_name].clone() {
                            let response = ui.add(
                                egui::Image::new(&texture)
                                    .fit_to_original_size(2.0)
                                    .max_height(140.0)
                                    .sense(egui::Sense::click()),
                            );
                            if response.clicked() {
                                self.modal_vars.modal_type = ModalType::ItemImage;
                            }
                        }
                    } else {
                        self.loaded_images.insert(image_name.to_owned(), None);
                        self.async_tasks_to_send
                            .push(CommandToServer::GetImageFromServer(
                                Uuid::new_v4().to_string(),
                                self.selected_item.id.clone(),
                                self.selected_item.image_type.clone(),
                                crate::database::data_helpers::ImageSize::Small,
                                ColorImage::default(),
                            ));
                    }
                }
                ui.heading(&self.selected_item.name);
            });
        });
        egui_extras::TableBuilder::new(ui)
            .striped(true)
            .column(Column::exact(180.0))
            .column(Column::remainder())
            .header(20.0, |mut header| {
                header.col(|ui| {
                    ui.label("");
                });
                header.col(|ui| {
                    ui.label("");
                });
            })
            .body(|mut body| {
                body.row(30.0, |mut row| {
                    row.col(|ui| {
                        ui.label("Name ");
                    });
                    row.col(|ui| {
                        if self.container_screen == ContainerScreen::EditingItem
                            || self.container_screen == ContainerScreen::AddingItem
                        {
                            ui.vertical_centered_justified(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.selected_item.name)
                                        .hint_text("Item Name"),
                                );
                            });
                        } else {
                            ui.horizontal(|ui| {
                                let clicked = Visualoc::interactive_label(
                                    ui,
                                    ctx,
                                    self.settings.light_mode,
                                    true,
                                    &self.selected_item.name,
                                );
                                if clicked {
                                    self.prepare_page(UIPages::Home);
                                    self.home_page.column_search =
                                        ("Name".to_string(), DataType::String);
                                    self.search_string = self.selected_item.name.clone();
                                }
                                Visualoc::copy_button(
                                    ui,
                                    self.settings.light_mode,
                                    self.selected_item.name.clone(),
                                    ctx,
                                );
                            });
                        }
                    });
                });
                let mut i64_index: usize = 0;
                let mut f64_index: usize = 0;
                let mut string_index: usize = 0;
                for (field_name, field_type) in self.item_field_types.clone() {
                    body.row(30.0, |mut row| {
                        row.col(|ui| {
                            ui.label(field_name.to_owned() + " ");
                        });
                        match field_type {
                            DataType::Float | DataType::Percentage => {
                                row.col(|ui| {
                                    self.edit_or_label(ui, field_type, field_name, f64_index, ctx);
                                });
                                f64_index += 1;
                            }
                            DataType::Integer | DataType::Bool => {
                                row.col(|ui| {
                                    self.edit_or_label(ui, field_type, field_name, i64_index, ctx);
                                });
                                i64_index += 1;
                            }
                            DataType::String
                            | DataType::Gallery
                            | DataType::Text
                            | DataType::List(_) => {
                                row.col(|ui| {
                                    self.edit_or_label(
                                        ui,
                                        field_type,
                                        field_name,
                                        string_index,
                                        ctx,
                                    );
                                });
                                string_index += 1;
                            }
                        }
                    });
                }
            });

        if self.container_screen == ContainerScreen::AddingItem
            || self.container_screen == ContainerScreen::EditingItem
        {
            let temp_string: String = if self.selected_item.image_type == String::new() {
                "➕ Add Image".to_string()
            } else {
                "Change Image".to_string()
            };
            if ui.button(temp_string).clicked() {
                self.loaded_images.remove(
                    &(self.selected_item.id.to_string() + "." + &self.selected_item.image_type),
                );
                self.async_tasks_to_send.push(CommandToServer::AddImage(
                    Uuid::new_v4().to_string(),
                    self.selected_item.id.clone(),
                    String::new(),
                ));
            }
        }
    }

    pub fn item_selected_screen(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        ScrollArea::vertical()
            .stick_to_right(true)
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| ui.heading("Item Details"));
                self.show_item_fields(ui, ctx);
                if self.container_screen == ContainerScreen::SelectedItem {
                    ui.separator();
                    ui.vertical_centered(|ui| ui.heading("Item Actions"));
                    ui.horizontal(|ui| {
                        if ui.button("✏ Edit Item").clicked() {
                            self.container_screen = ContainerScreen::EditingItem;
                        }
                        if Visualoc::delete_button(ui, "Delete Item").clicked() {
                            self.modal_vars.modal_type = ModalType::DeleteItem;
                        }
                        if ui.button("⬆ Remove from Container").clicked() {
                            self.modal_vars.modal_type = ModalType::RemoveFromContainer;
                        }
                    });
                }
                if self.container_screen == ContainerScreen::EditingItem {
                    ui.horizontal(|ui| {
                        if Visualoc::ok_button(ui).clicked() {
                            self.loaded_images.remove(
                                &(self.selected_item.id.to_string()
                                    + "."
                                    + &self.selected_item.image_type),
                            );
                            self.async_tasks_to_send.push(CommandToServer::UpdateItem(
                                Uuid::new_v4().to_string(),
                                self.selected_item.clone(),
                                self.item_field_types.clone(),
                            ));
                            self.async_tasks_to_send
                                .push(CommandToServer::GetMultipleItems(
                                    Uuid::new_v4().to_string(),
                                    self.selected_container.contained_items.clone(),
                                    Vec::new(),
                                ));
                            self.search_string = "".into();
                            self.item_page_search_vec = self.item_vec.clone();
                            self.container_screen = ContainerScreen::SelectedContainer;
                        }
                        if Visualoc::cancel_button(ui).clicked() {
                            self.loaded_images.remove(
                                &(self.selected_item.id.to_string()
                                    + "."
                                    + &self.selected_item.image_type),
                            );
                            self.async_tasks_to_send
                                .push(CommandToServer::GetMultipleItems(
                                    Uuid::new_v4().to_string(),
                                    self.selected_container.contained_items.clone(),
                                    Vec::new(),
                                ));
                            self.container_screen = ContainerScreen::SelectedContainer;
                        }
                    });
                }
            });
    }

    pub fn adding_item_screen(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        ui.vertical_centered(|ui| ui.heading("Adding Item"));
        ScrollArea::vertical()
            .stick_to_right(true)
            .auto_shrink(false)
            .show(ui, |ui| {
                self.show_item_fields(ui, ctx);
                ui.horizontal(|ui| {
                    if Visualoc::ok_button(ui).clicked() {
                        self.async_tasks_to_send.push(CommandToServer::UpdateItem(
                            Uuid::new_v4().to_string(),
                            self.selected_item.clone(),
                            self.item_field_types.clone(),
                        ));
                        self.selected_container
                            .contained_items
                            .insert(self.selected_item.id.clone());
                        self.item_vec.push(self.selected_item.clone());
                        self.item_vec.sort_by(|a, b| a.name.cmp(&b.name));
                        //Update the Container in the ContainerVec
                        for cont in &mut self.container_vec {
                            if cont.id == self.selected_container.id {
                                cont.contained_items.insert(self.selected_item.id.clone());
                            }
                        }
                        self.search_string = "".into();
                        self.item_page_search_vec = self.item_vec.clone();
                        self.container_screen = ContainerScreen::SelectedContainer;
                    }
                    if Visualoc::cancel_button(ui).clicked() {
                        self.async_tasks_to_send.push(CommandToServer::DeleteItem(
                            Uuid::new_v4().to_string(),
                            self.selected_item.clone(),
                            self.selected_container.id.clone(),
                            true,
                        ));
                        self.container_screen = ContainerScreen::SelectedContainer;
                    }
                });
            });
    }

    fn item_not_in_container_screen(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        ui.label(egui::RichText::new("Selected item is not in a container").color(Color32::RED));
        ScrollArea::vertical()
            .stick_to_right(true)
            .auto_shrink(false)
            .show(ui, |ui| {
                ui.vertical_centered(|ui| ui.heading("Item Details"));
                self.show_item_fields(ui, ctx);
                ui.separator();
                ui.vertical_centered(|ui| ui.heading("Item Actions"));
                if Visualoc::delete_button(ui, "Delete Item").clicked() {
                    self.modal_vars.modal_type = ModalType::DeleteItem;
                }
                ui.horizontal(|ui| {
                    if Visualoc::ok_button(ui).clicked() {
                        self.async_tasks_to_send.push(CommandToServer::UpdateItem(
                            Uuid::new_v4().to_string(),
                            self.selected_item.clone(),
                            self.item_field_types.clone(),
                        ));
                        self.prepare_page(UIPages::Home);
                    }
                    if Visualoc::cancel_button(ui).clicked() {
                        self.prepare_page(UIPages::Home);
                    }
                });
            });
    }

    pub fn location_containers_screen(&mut self, ui: &mut egui::Ui, ctx: &Context) {
        ui.with_layout(Layout::left_to_right(Align::LEFT), |ui| {
            self.draw_canvas(ui);
            ui.vertical(|ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Containers");
                });
                ui.with_layout(
                    Layout::left_to_right(Align::LEFT).with_main_wrap(true),
                    |ui| {
                        self.show_all_containers(ui);
                    },
                );
                ui.separator();
                ui.add_space(10.0);
                match self.container_screen {
                    ContainerScreen::None => self.initial_location_screen(ui),
                    ContainerScreen::EditingLocation => self.editing_location_screen(ui),
                    ContainerScreen::SelectedItem | ContainerScreen::EditingItem => {
                        self.item_selected_screen(ui, ctx)
                    }
                    ContainerScreen::AddingItem => self.adding_item_screen(ui, ctx),
                    ContainerScreen::AddingContainer | ContainerScreen::EditingContainer => {
                        self.add_edit_container_screen(ui)
                    }
                    ContainerScreen::SelectedContainer => self.container_selected_screen(ui),
                    ContainerScreen::ItemNotInContainer => {
                        self.item_not_in_container_screen(ui, ctx)
                    }
                }
                ui.separator();
                ui.add_space(10.0);
            });
        });
    }
}
