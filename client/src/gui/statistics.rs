use std::collections::HashMap;

use crate::{DataType, Visualoc};

impl Visualoc {
    pub fn statistics_screen(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            Visualoc::themed_heading(ui, self.settings.light_mode, "Locations in database:");
            ui.label(self.location_count.to_string());
        });
        ui.horizontal(|ui| {
            Visualoc::themed_heading(ui, self.settings.light_mode, "Items in database:");
            ui.label(self.item_count.to_string());
        });
        Visualoc::themed_heading(ui, self.settings.light_mode, "Most common values");
        for (index, (field_name, _)) in self.item_field_types.iter().enumerate() {
            ui.label(field_name);
            ui.separator();
            egui_extras::TableBuilder::new(ui)
                .id_salt(field_name)
                .striped(true)
                .columns(egui_extras::Column::remainder(), 2)
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.label("Value");
                    });
                    header.col(|ui| {
                        ui.label("Count");
                    });
                })
                .body(|mut body| {
                    for i in 0..std::cmp::min(5, self.max_min_field_values[index].len()) {
                        body.row(30.0, |mut row| {
                            row.col(|ui| {
                                ui.label(&self.max_min_field_values[index][i].0);
                            });
                            row.col(|ui| {
                                ui.label(self.max_min_field_values[index][i].1.to_string());
                            });
                        });
                    }
                });
            ui.separator();
            ui.add_space(30.0);
        }
        //Largest Container?
        //Container count
        //Location count
        //For each dynamic field 5 most and least common values
    }

    pub fn calculate_statistics(&mut self) {
        self.item_count = self.item_vec.len();
        self.location_count = self.container_vec.len();
        self.max_min_field_values = Vec::new();

        let mut f64_index = 0;
        let mut i64_index = 0;
        let mut string_index = 0;
        for (_, field_type) in &self.item_field_types {
            //Add each value to a hashmap
            let mut map: HashMap<String, usize> = HashMap::new();
            match field_type {
                DataType::Float => {
                    for item in &self.item_vec {
                        let key = item.float_vars[f64_index].to_string();
                        if key != String::default() {
                            match map.get(&key) {
                                Some(count) => map.insert(key, count + 1),
                                None => map.insert(key, 1),
                            };
                        }
                    }
                    f64_index += 1;
                }
                DataType::Integer => {
                    for item in &self.item_vec {
                        let key = item.int_vars[i64_index].to_string();
                        if key != String::default() {
                            match map.get(&key) {
                                Some(count) => map.insert(key, count + 1),
                                None => map.insert(key, 1),
                            };
                        }
                    }
                    i64_index += 1;
                }
                DataType::String => {
                    for item in &self.item_vec {
                        let key = item.string_vars[string_index].to_string();
                        if key != String::default() {
                            match map.get(&key) {
                                Some(count) => map.insert(key, count + 1),
                                None => map.insert(key, 1),
                            };
                        }
                    }
                    string_index += 1;
                }
                DataType::Gallery => {
                    string_index += 1;
                }
                DataType::List(_) => {
                    string_index += 1;
                }
                DataType::Percentage => {
                    f64_index += 1;
                }
                DataType::Bool => {
                    i64_index += 1;
                }
                DataType::Text => {
                    string_index += 1;
                }
            }
            //Hashmap to Vec
            let mut str_count_vec = Vec::new();
            for item in map {
                str_count_vec.push(item);
            }
            str_count_vec.sort_by(|a, b| b.1.cmp(&a.1));
            self.max_min_field_values.push(str_count_vec);
        }
    }
}
