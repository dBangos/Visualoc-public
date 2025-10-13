use std::cmp::Ordering;

use egui::{Color32, ColorImage, Margin, ScrollArea, Stroke};
use egui_extras::Column;
use uuid::Uuid;

use crate::{CommandToServer, DataType, ModalType, Visualoc};

impl Visualoc {
    pub fn home_page(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        //Search when the string is different
        if self.search_string != self.home_page.previous_search {
            self.home_page.previous_column_search = self.home_page.column_search.0.clone();
            self.home_page.previous_search = self.search_string.clone();
            self.home_page.page_number = 0;
            self.async_tasks_to_send.push(CommandToServer::SearchItems(
                Uuid::new_v4().to_string(),
                self.search_string.clone(),
                self.home_page.column_search.0.clone(),
                Vec::new(),
            ));
        }
        //If the vec holding the columns shown bools doesnt have the same len as the extra column, show all columns
        if self.item_fields_shown.len() != self.item_field_types.len() {
            self.item_fields_shown = vec![true; self.item_field_types.len()];
        }
        ui.add_space(5.0);
        Visualoc::themed_heading(
            ui,
            self.settings.light_mode,
            &format!("Searching in field: {}", self.home_page.column_search.0),
        );
        let (num_of_pages, first_shown_item, last_shown_item) = self.calculate_item_page_indexes();
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.search_string).hint_text("Search Items"));
            //Create a temp vec with the name added in front of item fields so the user can search each column
            let mut item_fields_types_plus_name: Vec<(String, DataType)> =
                vec![("Name".to_string(), DataType::String)];
            item_fields_types_plus_name.extend(self.item_field_types.clone());
            egui::ComboBox::from_id_salt("datatypesearchfield")
                .selected_text(&self.home_page.column_search.0)
                .show_ui(ui, |ui| {
                    for (column_name, column_type) in item_fields_types_plus_name {
                        ui.selectable_value(
                            &mut self.home_page.column_search,
                            (column_name.clone(), column_type.clone()),
                            column_name.clone(),
                        );
                    }
                });
            //If the column was changed search immediately
            if self.home_page.previous_column_search != self.home_page.column_search.0 {
                self.home_page.previous_search = self.search_string.clone();
                self.home_page.page_number = 0;
                self.home_page.previous_column_search = self.home_page.column_search.0.clone();
                self.async_tasks_to_send.push(CommandToServer::SearchItems(
                    Uuid::new_v4().to_string(),
                    self.search_string.clone(),
                    self.home_page.column_search.0.clone(),
                    Vec::new(),
                ));
            }

            ui.separator();
            ui.checkbox(&mut self.show_all_fields, "Show all fields");
            if !self.show_all_fields {
                if ui.button("Shown Fields").clicked() {
                    //Initialize bool vector if not initialized or not updated
                    if self.item_fields_shown.is_empty()
                        || self.item_fields_shown.len() != self.item_field_types.len()
                    {
                        self.item_fields_shown = vec![true; self.item_field_types.len()];
                    }
                    self.modal_vars.modal_type = ModalType::SelectFieldsShown;
                }
            } else {
                self.item_fields_shown = vec![true; self.item_field_types.len()];
            }
            ui.separator();
            //Navigation Buttons
            if ui.button("⏪").clicked() && self.home_page.page_number != 0 {
                for item in &self.item_vec[first_shown_item..last_shown_item] {
                    self.loaded_images
                        .remove(&(item.id.to_string() + "." + &item.image_type));
                }
                self.home_page.page_number = 0;
            }
            if ui.button("◀").clicked() && self.home_page.page_number > 0 {
                for item in &self.item_vec[first_shown_item..last_shown_item] {
                    self.loaded_images
                        .remove(&(item.id.to_string() + "." + &item.image_type));
                }
                self.home_page.page_number -= 1;
            }
            ui.label(
                (self.home_page.page_number + 1).to_string() + " / " + &num_of_pages.to_string(),
            );
            if ui.button("▶").clicked() && self.home_page.page_number < num_of_pages - 1 {
                for item in &self.item_vec[first_shown_item..last_shown_item] {
                    self.loaded_images
                        .remove(&(item.id.to_string() + "." + &item.image_type));
                }
                self.home_page.page_number += 1;
            }
            if ui.button("⏩").clicked() && self.home_page.page_number != num_of_pages - 1 {
                for item in &self.item_vec[first_shown_item..last_shown_item] {
                    self.loaded_images
                        .remove(&(item.id.to_string() + "." + &item.image_type));
                }
                self.home_page.page_number = num_of_pages - 1;
            }
        });
        //Calculate the page indexes again in case they have changed form the search being called
        let (_, first_shown_item, last_shown_item) = self.calculate_item_page_indexes();
        egui::frame::Frame::default()
            .stroke(Stroke {
                width: 2.0,
                color: Color32::DARK_GRAY,
            })
            .inner_margin(Margin {
                left: 10,
                right: 20,
                top: 10,
                bottom: 10,
            })
            .show(ui, |ui| {
                ScrollArea::horizontal().show(ui, |ui| {
                    let mut table = egui_extras::TableBuilder::new(ui);
                    //Keybinds
                    if ctx.input(|i| i.key_pressed(egui::Key::Home)) && !self.item_vec.is_empty() {
                        table = table.scroll_to_row(0, Some(egui::Align::TOP));
                    }
                    if ctx.input(|i| i.key_pressed(egui::Key::End)) && !self.item_vec.is_empty() {
                        table =
                            table.scroll_to_row(self.item_vec.len() - 1, Some(egui::Align::BOTTOM));
                    }
                    //Show the table
                    table
                        .striped(true)
                        .column(Column::initial(180.0).resizable(true))//Image column has default width so it's defined separately
                        .columns(
                            egui_extras::Column::remainder().resizable(true),
                            // Name + Dynamic fields shown
                            self.item_fields_shown.iter().filter(|x| **x).count() + 1,
                        )
                        .header(20.0, |mut header| {
                            header.col(|ui| {
                                Visualoc::themed_heading(ui, self.settings.light_mode, "Image");
                            });
                            header.col(|ui| {
                                ui.horizontal(|ui| {
                                    let name_response = Visualoc::themed_heading(
                                        ui,
                                        self.settings.light_mode,
                                        "Name",
                                    );
                                    if self.home_page.search_results_sorted && name_response.hovered() {
                                        Visualoc::themed_heading(ui, self.settings.light_mode, "⏶");
                                    } else if !self.home_page.search_results_sorted && name_response.hovered()
                                    {
                                        Visualoc::themed_heading(ui, self.settings.light_mode, "⏷");
                                    }
                                    if name_response.clicked() {
                                        //Sort the results by name, reverse if already sorted
                                        if self.home_page.search_results_sorted {
                                            self.item_vec.sort_unstable_by_key(|x| x.name.clone());
                                            self.item_vec.reverse();
                                            self.home_page.search_results_sorted = false;
                                        } else {
                                            self.item_vec.sort_unstable_by_key(|x| x.name.clone());
                                            self.home_page.search_results_sorted = true;
                                        }
                                    }
                                });
                            });
                            for (index, (column_name, _)) in
                                self.item_field_types.iter().enumerate()
                            {
                                if self.item_fields_shown[index] {
                                    header.col(|ui| {
                                        ui.horizontal(|ui| {
                                            //If the column title is clicked sort the data
                                            let name_response = Visualoc::themed_heading(
                                                ui,
                                                self.settings.light_mode,
                                                column_name,
                                            );
                                            if self.home_page.search_results_sorted && name_response.hovered()
                                            {
                                                Visualoc::themed_heading(
                                                    ui,
                                                    self.settings.light_mode,
                                                    "⏶",
                                                );
                                            } else if !self.home_page.search_results_sorted
                                                && name_response.hovered()
                                            {
                                                Visualoc::themed_heading(
                                                    ui,
                                                    self.settings.light_mode,
                                                    "⏷",
                                                );
                                            }
                                            if name_response.clicked() {
                                                //Get the proper vector and the position from the column name
                                                let mut column_data_type: DataType =
                                                    DataType::String;
                                                let mut column_index = 0;
                                                let mut string_index = 0;
                                                let mut f64_index = 0;
                                                let mut i64_index = 0;
                                                for (current_column_name, column_type) in
                                                    self.item_field_types.clone()
                                                {
                                                    match column_type {
                                                        DataType::String => {
                                                            if *column_name == current_column_name {
                                                                column_data_type = DataType::String;
                                                                column_index = string_index;
                                                                break;
                                                            }
                                                            string_index += 1;
                                                        }
                                                        DataType::Integer => {
                                                            if *column_name == current_column_name {
                                                                column_data_type =
                                                                    DataType::Integer;
                                                                column_index = i64_index;
                                                                break;
                                                            }
                                                            i64_index += 1;
                                                        }
                                                        DataType::Float => {
                                                            if *column_name == current_column_name {
                                                                column_data_type = DataType::Float;
                                                                column_index = f64_index;
                                                                break;
                                                            }
                                                            f64_index += 1;
                                                        }
                                                        DataType::Bool=>{
                                                            if *column_name == current_column_name {
                                                                column_data_type = DataType::Bool;
                                                                column_index = i64_index;
                                                                break;
                                                            }
                                                            i64_index += 1;
                                                        }
                                                        DataType::Text=>{
                                                            if *column_name == current_column_name {
                                                                column_data_type = DataType::Text;
                                                                column_index = string_index;
                                                                break;
                                                            }
                                                            string_index += 1;
                                                        }
                                                        DataType::List(val)=>{
                                                            if *column_name == current_column_name {
                                                                column_data_type = DataType::List(val);
                                                                column_index = string_index;
                                                                break;
                                                            }
                                                            string_index += 1;
                                                        }
                                                        DataType::Gallery=>{
                                                            if *column_name == current_column_name {
                                                                column_data_type = DataType::Gallery;
                                                                column_index = string_index;
                                                                break;
                                                            }
                                                            string_index += 1;
                                                        }
                                                        DataType::Percentage=>{
                                                            if *column_name == current_column_name {
                                                                column_data_type = DataType::Percentage;
                                                                column_index = f64_index;
                                                                break;
                                                            }
                                                            f64_index += 1;
                                                        }
                                                    }
                                                }
                                                //Sort the results, reverse if already sorted
                                                match column_data_type {
                                                    DataType::String | DataType::List(_) | DataType::Text=> {
                                                        self.item_vec.sort_unstable_by_key(|x| {
                                                            x.string_vars[column_index].clone()
                                                        })
                                                    }
                                                    DataType::Integer | DataType::Bool => {
                                                        self.item_vec.sort_unstable_by_key(|x| {
                                                            x.int_vars[column_index]
                                                        })
                                                    }
                                                    DataType::Float | DataType::Percentage => {
                                                        self.item_vec.sort_by(|a, b| {
                                                            match a.float_vars[column_index]
                                                                .partial_cmp(
                                                                    &b.float_vars[column_index],
                                                                ) {
                                                                Some(ordering) => ordering,
                                                                None => Ordering::Equal,
                                                            }
                                                        });
                                                    }
                                                    DataType::Gallery=>()
                                                }
                                                if self.home_page.search_results_sorted {
                                                    self.item_vec.reverse();
                                                    self.home_page.search_results_sorted = false;
                                                } else {
                                                    self.home_page.search_results_sorted = true;
                                                }
                                            }
                                        });
                                    });
                                }
                            }
                        })
                        .body(|mut body| {
                            //Show the rows
                            for item in &self.item_vec[first_shown_item..last_shown_item] {
                                let mut line_height = 30.0;
                                if item.image_type != String::default() {
                                    line_height = 70.0;
                                }
                                body.row(line_height, |mut row| {
                                    let mut clicked = false;
                                    row.col(|ui| {
                                        ui.vertical_centered(|ui| {
                                            if item.image_type != String::default() {
                                                let image_name =
                                                    &(item.id.clone() + "." + &item.image_type);
                                                if self.loaded_images.contains_key(image_name) {
                                                    if let Some((texture,_)) =
                                                        self.loaded_images[image_name].clone()
                                                    {
                                                        let image_response = ui.add(
                                                            egui::Image::new(&texture)
                                                                .fit_to_original_size(2.0)
                                                                .max_height(line_height)
                                                                .sense(egui::Sense::click()),
                                                        );
                                                        if image_response.clicked() {
                                                            clicked = true;
                                                        }
                                                    }
                                                } else {
                                                    self.loaded_images
                                                        .insert(image_name.to_owned(), None);
                                                    self.async_tasks_to_send.push(
                                                        CommandToServer::GetImageFromServer(
                                                            Uuid::new_v4().to_string(),
                                                            item.id.clone(),
                                                            item.image_type.clone(),
                                                            crate::database::data_helpers::ImageSize::Small,
                                                            ColorImage::default(),
                                                        ),
                                                    );
                                                }
                                            }
                                        });
                                    });
                                    row.col(|ui| {
                                        ui.add_space(ui.available_size().y / 3.0);
                                        ui.horizontal(|ui| {
                                            ui.add_space(ui.available_size().x / 5.0);
                                            let label_clicked = Visualoc::interactive_label(
                                                ui,
                                                ctx,
                                                self.settings.light_mode,
                                                false,
                                                &item.name,
                                            );
                                            if label_clicked {
                                                clicked = true;
                                            }
                                        });
                                    });
                                    let mut string_index = 0;
                                    let mut f64_index = 0;
                                    let mut i64_index = 0;
                                    for (index, (_, column_type)) in
                                        self.item_field_types.iter().enumerate()
                                    {
                                        if self.item_fields_shown[index] {
                                            row.col(|ui| {
                                                ui.centered_and_justified(|ui| match column_type {
                                                    DataType::String |DataType::List(_) | DataType::Text => {
                                                        ui.add_space(ui.available_size().y / 3.0);
                                                        ui.horizontal(|ui| {
                                                            ui.add_space(
                                                                ui.available_size().x / 5.0,
                                                            );
                                                            let label_clicked =
                                                                Visualoc::interactive_label(
                                                                    ui,
                                                                    ctx,
                                                                    self.settings.light_mode,
                                                                    false,
                                                                    &item.string_vars[string_index]
                                                                );

                                                            if label_clicked {
                                                                clicked = true;
                                                            }
                                                        });
                                                        string_index += 1;
                                                    }
                                                    DataType::Gallery=>{
                                                        ui.add_space(ui.available_size().y / 3.0);
                                                        ui.horizontal(|ui| {
                                                            ui.add_space(
                                                                ui.available_size().x / 5.0,
                                                            );
                                                            let label_clicked =
                                                                Visualoc::interactive_label(
                                                                    ui,
                                                                    ctx,
                                                                    self.settings.light_mode,
                                                                    false,
                                                                    &"todo".to_string()
                                                                );

                                                            if label_clicked {
                                                                clicked = true;
                                                            }
                                                        });
                                                        string_index += 1;
                                                    }
                                                    DataType::Integer => {
                                                        ui.add_space(ui.available_size().y / 3.0);
                                                        ui.horizontal(|ui| {
                                                            ui.add_space(
                                                                ui.available_size().x / 5.0,
                                                            );
                                                            let label_clicked =
                                                                Visualoc::interactive_label(
                                                                    ui,
                                                                    ctx,
                                                                    self.settings.light_mode,
                                                                    false,
                                                                    &item.int_vars[i64_index]
                                                                        .to_string(),
                                                                );

                                                            if label_clicked {
                                                                clicked = true;
                                                            }
                                                        });
                                                        i64_index += 1;
                                                    }
                                                    DataType::Float => {
                                                        ui.add_space(ui.available_size().y / 3.0);
                                                        ui.horizontal(|ui| {
                                                            ui.add_space(
                                                                ui.available_size().x / 5.0,
                                                            );
                                                            let label_clicked =
                                                                Visualoc::interactive_label(
                                                                    ui,
                                                                    ctx,
                                                                    self.settings.light_mode,
                                                                    false,
                                                                    &item.float_vars[f64_index]
                                                                        .to_string(),
                                                                );

                                                            if label_clicked {
                                                                clicked = true;
                                                            }
                                                        });
                                                        f64_index += 1;
                                                    }
                                                    DataType::Bool=>{
                                                        ui.add_space(ui.available_size().y / 3.0);
                                                        ui.horizontal(|ui| {
                                                            ui.add_space(
                                                                ui.available_size().x / 5.0,
                                                            );
                                                            let symbol = if item.int_vars[i64_index] == 0 { "✖" } else { "✅" };
                                                            let label_clicked =
                                                                Visualoc::interactive_label(
                                                                    ui,
                                                                    ctx,
                                                                    self.settings.light_mode,
                                                                    false,
                                                                    &symbol.to_string()
                                                                );

                                                            if label_clicked {
                                                                clicked = true;
                                                            }
                                                        });
                                                        i64_index+= 1;
                                                    }
                                                    DataType::Percentage=>{
                                                        ui.add_space(ui.available_size().y / 3.0);
                                                        ui.horizontal(|ui| {
                                                            ui.add_space(
                                                                ui.available_size().x / 5.0,
                                                            );
                                                            let percentage_string = &(item.float_vars[f64_index].to_string() + "%");
                                                            let label_clicked =
                                                                Visualoc::interactive_label(
                                                                    ui,
                                                                    ctx,
                                                                    self.settings.light_mode,
                                                                    false,
                                                                    percentage_string
                                                                );

                                                            if label_clicked {
                                                                clicked = true;
                                                            }
                                                        });
                                                        f64_index+= 1;
                                                    }
                                                });
                                            });
                                        }
                                    }
                                    if clicked {
                                        self.async_tasks_to_send.push(
                                            CommandToServer::GetItemLocationContainer(
                                                Uuid::new_v4().to_string(),
                                                item.id.clone(),
                                                None,
                                            ),
                                        );
                                        self.selected_item = item.clone();
                                    }
                                });
                            }
                        });
                });
            });
    }
}
