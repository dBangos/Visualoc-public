use egui::{Color32, Modal, Widget};
use uuid::Uuid;

use crate::{
    CommandToServer, DataType, FieldModal, ModalType, Visualoc, WaitingFunction,
    WaitingFunctionKind,
};

impl Visualoc {
    pub fn dynamic_field_edit_modal(&mut self, ctx: &egui::Context) {
        Modal::new(self.modal_vars.field_modal_id.clone().into()).show(ctx, |ui| {
            Visualoc::themed_heading(ui, self.settings.light_mode, "Edit Item Fields");
            ui.separator();
            ui.add_space(20.0);
            if !self.item_field_types.is_empty() {
                let mut column_number = 2;
                if self.modal_vars.field_modal == FieldModal::DeletingField
                    || self.modal_vars.field_modal == FieldModal::EditingField(false)
                {
                    column_number = 3;
                }
                egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .columns(egui_extras::Column::auto().at_least(200.0), column_number)
                    .header(20.0, |mut header| {
                        if self.modal_vars.field_modal == FieldModal::DeletingField
                            || self.modal_vars.field_modal == FieldModal::EditingField(false)
                        {
                            header.col(|ui| {
                                Visualoc::themed_heading(ui, self.settings.light_mode, "");
                            });
                        }
                        header.col(|ui| {
                            Visualoc::themed_heading(ui, self.settings.light_mode, "Field Name");
                        });
                        header.col(|ui| {
                            Visualoc::themed_heading(ui, self.settings.light_mode, "Field Type");
                        });
                    })
                    .body(|mut body| {
                        for (index, (column_name, column_type)) in
                            self.item_field_types.iter().enumerate()
                        {
                            body.row(30.0, |mut row| {
                                if self.modal_vars.field_modal == FieldModal::DeletingField
                                    || self.modal_vars.field_modal
                                        == FieldModal::EditingField(false)
                                {
                                    row.col(|ui| {
                                        ui.checkbox(
                                            &mut self.modal_vars.item_field_selected_fields[index],
                                            "",
                                        );
                                    });
                                }
                                row.col(|ui| {
                                    ui.label(column_name);
                                });
                                row.col(|ui| match column_type {
                                    DataType::String => {
                                        ui.label("Text");
                                    }
                                    DataType::Float => {
                                        ui.label("Decimal");
                                    }
                                    DataType::Integer => {
                                        ui.label("Integer");
                                    }
                                    DataType::Bool => {
                                        ui.label("Checkbox");
                                    }
                                    DataType::List(_) => {
                                        ui.label("List");
                                    }
                                    DataType::Text => {
                                        ui.label("Paragraph");
                                    }
                                    DataType::Gallery => {
                                        ui.label("Image Gallery");
                                    }
                                    DataType::Percentage => {
                                        ui.label("Percentage");
                                    }
                                });
                            });
                        }
                    });
                ui.add_space(15.0);
                ui.separator();
            }
            match self.modal_vars.field_modal {
                FieldModal::Start => {
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() / 2.0 - 190.0);
                        if Visualoc::add_button(ui, "Add Field").clicked() {
                            self.modal_vars.field_modal = FieldModal::AddingField;
                        }
                        if Visualoc::delete_button(ui, "Delete Field").clicked()
                            && !self.item_field_types.is_empty()
                        {
                            self.modal_vars.item_field_selected_fields =
                                vec![false; self.item_field_types.len()];
                            self.modal_vars.field_modal = FieldModal::DeletingField;
                        }
                        if ui.button("✏ Edit Field").clicked() && !self.item_field_types.is_empty()
                        {
                            self.modal_vars.item_field_selected_fields =
                                vec![false; self.item_field_types.len()];
                            self.modal_vars.field_modal = FieldModal::EditingField(false);
                        }
                    });
                }
                FieldModal::DeletingField => {
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() / 2.0 - 100.0);
                        if Visualoc::ok_button(ui).clicked()
                            && self
                                .modal_vars
                                .item_field_selected_fields
                                .iter()
                                .any(|x| *x)
                        {
                            self.modal_vars.modal_type = ModalType::DeleteField;
                        }
                        if Visualoc::cancel_button(ui).clicked() {
                            self.modal_vars.field_modal = FieldModal::Start;
                            self.modal_vars.field_modal_id = Uuid::new_v4().to_string();
                        }
                    });
                }
                FieldModal::EditingField(_) => {
                    if self.modal_vars.field_modal == FieldModal::EditingField(false) {
                        ui.label("Select a field to edit");
                        if self
                            .modal_vars
                            .item_field_selected_fields
                            .iter()
                            .filter(|x| **x)
                            .count()
                            == 1
                        {
                            for (index, selected) in self
                                .modal_vars
                                .item_field_selected_fields
                                .iter()
                                .enumerate()
                            {
                                if *selected {
                                    (
                                        self.modal_vars.new_field_name,
                                        self.modal_vars.new_field_type,
                                    ) = self.item_field_types[index].clone();
                                    //If it is a list use the first field to hold the joined list
                                    match &mut self.modal_vars.new_field_type {
                                        DataType::List(string_vec) => {
                                            string_vec[0] = string_vec.join(",");
                                            if string_vec.is_empty() {
                                                string_vec.push(String::default());
                                            }
                                        }
                                        _ => (),
                                    }
                                    break;
                                }
                            }
                            self.modal_vars.field_modal = FieldModal::EditingField(true);
                            self.modal_vars.field_modal_id = Uuid::new_v4().to_string();
                        }
                    } else if self.modal_vars.field_modal == FieldModal::EditingField(true) {
                        ui.vertical_centered(|ui| {
                            let text_response =
                                egui::TextEdit::singleline(&mut self.modal_vars.new_field_name)
                                    .hint_text("Field Name")
                                    .ui(ui);
                            if !self
                                .modal_vars
                                .new_field_name
                                .chars()
                                .all(|x| x.is_alphanumeric())
                            {
                                ui.colored_label(
                                    Color32::RED,
                                    "Name can contain only letters and numbers",
                                );
                                text_response.has_focus();
                            }
                            if self.modal_vars.new_field_name.is_empty() {
                                ui.colored_label(Color32::RED, "Name can not be empty");
                                text_response.has_focus();
                            }
                            //Handling lists separately since they are the only data type with type content(Vec<String>)
                            match &mut self.modal_vars.new_field_type {
                                DataType::List(string_vec) => {
                                    ui.label("Add the list values separated by a comma (,)");
                                    //The existence of string_vec[0] is guaranteed by the checkbox click
                                    ui.text_edit_multiline(&mut string_vec[0]);
                                }
                                _ => (),
                            }
                        });
                        ui.add_space(5.0);
                        ui.horizontal(|ui| {
                            ui.add_space(ui.available_width() / 2.0 - 100.0);
                            if Visualoc::ok_button(ui).clicked()
                                && self
                                    .modal_vars
                                    .new_field_name
                                    .chars()
                                    .all(|x| x.is_alphanumeric())
                                && !self.modal_vars.new_field_name.is_empty()
                                && self.modal_vars.new_field_name != "id"
                                && self.modal_vars.new_field_name != "name"
                                && self.modal_vars.new_field_name != "image_type"
                            {
                                for (index, selected) in self
                                    .modal_vars
                                    .item_field_selected_fields
                                    .iter()
                                    .enumerate()
                                {
                                    if *selected {
                                        //If it is a list type create the vec
                                        match &mut self.modal_vars.new_field_type {
                                            DataType::List(string_vec) => {
                                                let string_vec: Vec<String> = string_vec[0]
                                                    .trim()
                                                    .split(",")
                                                    .map(|x| String::from(x))
                                                    .collect();
                                                self.modal_vars.new_field_type =
                                                    DataType::List(string_vec);
                                            }
                                            _ => (),
                                        }
                                        //Update the database
                                        self.async_tasks_to_send.push(
                                            CommandToServer::UpdateItemsColumn(
                                                Uuid::new_v4().to_string(),
                                                (
                                                    self.modal_vars.new_field_name.clone(),
                                                    self.modal_vars.new_field_type.clone(),
                                                ),
                                                self.item_field_types[index].0.clone(),
                                            ),
                                        );
                                        //Update in memory
                                        self.item_field_types[index] = (
                                            self.modal_vars.new_field_name.clone(),
                                            self.modal_vars.new_field_type.clone(),
                                        );
                                        break;
                                    }
                                }

                                self.modal_vars.field_modal = FieldModal::Start;
                            }
                            if Visualoc::cancel_button(ui).clicked() {
                                self.modal_vars.new_field_name = String::new();
                                self.modal_vars.field_modal = FieldModal::Start;
                            }
                        });
                    }

                    ui.add_space(20.0);
                    ui.separator();
                    ui.vertical_centered(|ui| {
                        if ui.button("⬅ Back").clicked() {
                            self.modal_vars.field_modal = FieldModal::Start;
                            self.modal_vars.field_modal_id = Uuid::new_v4().to_string();
                        }
                    });
                }
                FieldModal::AddingField => {
                    let text_response =
                        egui::TextEdit::singleline(&mut self.modal_vars.new_field_name)
                            .hint_text("Field Name")
                            .ui(ui);
                    if !self
                        .modal_vars
                        .new_field_name
                        .chars()
                        .all(|x| x.is_alphanumeric())
                    {
                        ui.colored_label(Color32::RED, "Name can contain only letters and numbers");
                        text_response.has_focus();
                    }
                    if self.modal_vars.new_field_name.is_empty() {
                        ui.colored_label(Color32::RED, "Name can not be empty");
                        text_response.has_focus();
                    }
                    ui.horizontal(|ui| {
                        ui.label("Type: ");
                        egui::ComboBox::from_id_salt("datatypeFieldModal")
                            .selected_text(match self.modal_vars.new_field_type {
                                DataType::Float => "Decimal",
                                DataType::Integer => "Integer",
                                DataType::String => "Text",
                                DataType::Bool => "Checkbox",
                                DataType::List(_) => "List",
                                DataType::Text => "Paragraph",
                                DataType::Gallery => "Image Gallery",
                                DataType::Percentage => "Percentage",
                            })
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut self.modal_vars.new_field_type,
                                    DataType::String,
                                    "Text",
                                );
                                ui.selectable_value(
                                    &mut self.modal_vars.new_field_type,
                                    DataType::Integer,
                                    "Integer",
                                );
                                ui.selectable_value(
                                    &mut self.modal_vars.new_field_type,
                                    DataType::Float,
                                    "Decimal",
                                );
                                ui.selectable_value(
                                    &mut self.modal_vars.new_field_type,
                                    DataType::Bool,
                                    "Checkbox",
                                );
                                ui.selectable_value(
                                    &mut self.modal_vars.new_field_type,
                                    DataType::List(Vec::new()),
                                    "List",
                                );
                                ui.selectable_value(
                                    &mut self.modal_vars.new_field_type,
                                    DataType::Text,
                                    "Paragraph",
                                );
                                ui.selectable_value(
                                    &mut self.modal_vars.new_field_type,
                                    DataType::Percentage,
                                    "Percentage",
                                );
                                ui.selectable_value(
                                    &mut self.modal_vars.new_field_type,
                                    DataType::Gallery,
                                    "Image Gallery",
                                );
                            });
                    });
                    ui.separator();
                    ui.add_space(20.0);

                    match &mut self.modal_vars.new_field_type {
                        DataType::List(string_vec) => {
                            if string_vec.is_empty() {
                                string_vec.push(String::default());
                            }
                            ui.label("Add the list values separated by a comma (,)");
                            ui.text_edit_multiline(&mut string_vec[0]);
                        }
                        _ => (),
                    }
                    ui.separator();
                    ui.add_space(20.0);
                    ui.horizontal(|ui| {
                        ui.add_space(ui.available_width() / 2.0 - 100.0);
                        if Visualoc::ok_button(ui).clicked()
                            && self
                                .modal_vars
                                .new_field_name
                                .chars()
                                .all(|x| x.is_alphanumeric())
                            && !self.modal_vars.new_field_name.is_empty()
                            && self.modal_vars.new_field_name != "id"
                            && self.modal_vars.new_field_name != "name"
                            && self.modal_vars.new_field_name != "image_type"
                        {
                            let cmd_id = Uuid::new_v4().to_string();
                            self.async_tasks_to_send.push(CommandToServer::AddField(
                                cmd_id.clone(),
                                self.modal_vars.new_field_name.clone(),
                                self.modal_vars.new_field_type.clone(),
                            ));
                            //Change the already loaded items to include the extra field
                            match &mut self.modal_vars.new_field_type {
                                DataType::Integer | DataType::Bool => {
                                    for item in self.item_vec.iter_mut() {
                                        item.int_vars.push(0);
                                    }
                                }
                                DataType::Float | DataType::Percentage => {
                                    for item in self.item_vec.iter_mut() {
                                        item.float_vars.push(0.0);
                                    }
                                }
                                DataType::String | DataType::Text | DataType::Gallery => {
                                    for item in self.item_vec.iter_mut() {
                                        item.string_vars.push(String::new());
                                    }
                                }
                                DataType::List(string_vec) => {
                                    //The user input is stored in the first field of the vec
                                    //This parses the string and creates the list vector
                                    let temp = &string_vec[0].clone();
                                    let temp: Vec<&str> = temp.trim().split(",").collect();
                                    string_vec[0] = String::new();
                                    for string in temp {
                                        string_vec.push(string.to_string());
                                    }
                                    //Initialize the field for all items like in all other data types
                                    for item in self.item_vec.iter_mut() {
                                        item.string_vars.push(String::new());
                                    }
                                }
                            }
                            self.functions_waiting_data.push(WaitingFunction {
                                id: cmd_id,
                                kind: WaitingFunctionKind::AddFieldOk,
                            });
                        }
                        if Visualoc::cancel_button(ui).clicked() {
                            self.modal_vars.new_field_name = String::new();
                            self.modal_vars.field_modal = FieldModal::Start;
                        }
                    });
                }
                FieldModal::None => (),
            }
            if self.modal_vars.field_modal == FieldModal::Start {
                ui.separator();
                ui.add_space(15.0);
                ui.vertical_centered(|ui| {
                    if ui.button("✖ Close").clicked() {
                        self.modal_vars.field_modal = FieldModal::None;
                        self.modal_vars.field_modal_id = Uuid::new_v4().to_string();
                    }
                });
            }
        });
    }
}
