use egui::{Button, Color32, Context, Label, Rect, Response, RichText, Sense, Stroke, Ui, Vec2};
use uuid::Uuid;

use crate::{
    CommandToServer, Container, ContainerScreen, DataType, FieldModal, ModalType, UIPages,
    Visualoc, WaitingFunction, WaitingFunctionKind, toggle_light_mode,
};

impl Visualoc {
    pub fn calculate_item_page_indexes(&mut self) -> (usize, usize, usize) {
        let num_of_pages: usize = if self.item_vec.len() % self.settings.items_per_page == 0 {
            if self.item_vec.is_empty() {
                1
            } else {
                self.item_vec.len() / self.settings.items_per_page
            }
        } else {
            self.item_vec.len() / self.settings.items_per_page + 1
        };

        let first_shown_item = self.home_page.page_number * self.settings.items_per_page;

        let last_shown_item: usize =
            if first_shown_item + self.settings.items_per_page - 1 < self.item_vec.len() {
                first_shown_item + self.settings.items_per_page - 1
            } else if !self.item_vec.is_empty() {
                self.item_vec.len()
            } else {
                first_shown_item
            };
        return (num_of_pages, first_shown_item, last_shown_item);
    }

    pub fn calculate_label_size_with_wrap(ui: &Ui, text: &str, max_width: f32) -> egui::Rect {
        let font_id = ui.style().text_styles.get(&egui::TextStyle::Body).unwrap();
        let galley = ui.fonts(|f| {
            f.layout(
                text.to_string(),
                font_id.clone(),
                ui.visuals().text_color(),
                max_width, // Wrap at this width
            )
        });
        galley.rect
    }

    pub fn interactive_label(
        ui: &mut egui::Ui,
        ctx: &Context,
        light_mode: bool,
        wrap: bool,
        text: &String,
    ) -> bool {
        let text_rect = Visualoc::calculate_label_size_with_wrap(ui, text, ui.available_width());

        let rect: Rect = if wrap {
            Rect {
                min: ui.cursor().min,
                max: ui.cursor().min
                    + Vec2 {
                        x: text_rect.max.x,
                        y: text_rect.max.y,
                    },
            }
        } else {
            // If the wrap is inactive, constrain the y to 30.0, which is close to a single row
            // This way the text truncates properly
            Rect {
                min: ui.cursor().min,
                max: ui.cursor().min
                    + Vec2 {
                        x: text_rect.max.x,
                        y: 30.0,
                    },
            }
        };
        if let Some(pos) = ctx.input(|i| i.pointer.hover_pos()) {
            if pos.x >= rect.min.x
                && pos.x <= rect.max.x
                && pos.y >= rect.min.y
                && pos.y <= rect.max.y
            {
                if light_mode {
                    if wrap {
                        ui.put(
                            rect,
                            Label::new(RichText::new(text).color(Color32::BLACK).underline())
                                .wrap(),
                        );
                    } else {
                        ui.put(
                            rect,
                            Label::new(RichText::new(text).color(Color32::BLACK).underline())
                                .truncate(),
                        );
                    }
                } else {
                    if wrap {
                        ui.put(
                            rect,
                            Label::new(RichText::new(text).color(Color32::WHITE).underline())
                                .wrap(),
                        );
                    } else {
                        ui.put(
                            rect,
                            Label::new(RichText::new(text).color(Color32::WHITE).underline())
                                .truncate(),
                        );
                    }
                }
                return ctx.input(|i| i.pointer.any_click());
            } else {
                if wrap {
                    ui.put(rect, Label::new(RichText::new(text)).wrap());
                } else {
                    ui.put(rect, Label::new(RichText::new(text)).truncate());
                }
            }
        } else {
            if wrap {
                ui.put(rect, Label::new(RichText::new(text)).wrap());
            } else {
                ui.put(rect, Label::new(RichText::new(text)).truncate());
            }
        }
        return false;
    }

    pub fn copy_button(
        ui: &mut Ui,
        light_mode: bool,
        text: String,
        ctx: &egui::Context,
    ) -> Response {
        let response = ui.interact(
            ui.available_rect_before_wrap(),
            ui.next_auto_id(),
            Sense::hover() | Sense::click(),
        );
        if response.hovered() {
            if light_mode {
                ui.add(
                    egui::Image::new(egui::include_image!(
                        "../../assets/copy-link-icon-black.png"
                    ))
                    .sense(Sense::click()),
                );
            } else {
                ui.add(
                    egui::Image::new(egui::include_image!(
                        "../../assets/copy-link-icon-white.png"
                    ))
                    .sense(Sense::click()),
                );
            }
        } else if light_mode {
            ui.add(
                egui::Image::new(egui::include_image!(
                    "../../assets/copy-link-icon-dark-grey.png"
                ))
                .sense(Sense::click()),
            );
        } else {
            ui.add(
                egui::Image::new(egui::include_image!("../../assets/copy-link-icon.png"))
                    .sense(Sense::click()),
            );
        }

        if response.clicked() {
            ctx.copy_text(text);
        }
        return response;
    }

    pub fn delete_button(ui: &mut Ui, text: &str) -> Response {
        let button = Button::new("✖".to_owned() + text);
        ui.style_mut().visuals.widgets.hovered.bg_stroke = Stroke {
            color: Color32::from_rgb(150, 6, 40),
            width: 1.0,
        };
        let response = ui.add(button);
        return response;
    }

    pub fn add_button(ui: &mut Ui, text: &str) -> Response {
        let button = Button::new("➕ ".to_owned() + text);
        let response = ui.add(button);
        return response;
    }

    pub fn cancel_button(ui: &mut Ui) -> Response {
        let button = Button::new("✖ Cancel".to_owned());
        ui.style_mut().visuals.widgets.hovered.bg_stroke = Stroke {
            color: Color32::RED,
            width: 1.0,
        };
        let response = ui.add(button);
        return response;
    }

    pub fn ok_button(ui: &mut Ui) -> Response {
        let button = Button::new("✔     Ok    ".to_owned());
        ui.style_mut().visuals.widgets.hovered.bg_stroke = Stroke {
            color: Color32::GREEN,
            width: 1.0,
        };
        let response = ui.add(button);
        return response;
    }

    pub fn initialize(&mut self, ctx: &egui::Context) {
        self.async_tasks_to_send
            .push(CommandToServer::GetItemColumnTypes(
                Uuid::new_v4().to_string(),
                Vec::new(),
            ));
        toggle_light_mode(ctx, self.settings.light_mode);
    }

    pub fn dynamic_fields_initialization(&mut self) {
        //Adjust the vectors containing the data from the user added fields
        //for items so that their size and initial values are ok to be used for item creation
        for (_, column_type) in &self.item_field_types {
            match column_type {
                DataType::String | DataType::List(_) | DataType::Text | DataType::Gallery => {
                    self.selected_item.string_vars.push(String::new())
                }
                DataType::Integer | DataType::Bool => self.selected_item.int_vars.push(0),
                DataType::Float | DataType::Percentage => self.selected_item.float_vars.push(0.0),
            }
        }
    }

    pub fn prepare_page(&mut self, next_page: UIPages) {
        match next_page {
            UIPages::Home => {
                self.async_tasks_to_send
                    .push(CommandToServer::GetItemColumnTypes(
                        Uuid::new_v4().to_string(),
                        Vec::new(),
                    ));
                self.home_page.page_number = 0;
                self.current_ui = UIPages::Home;
            }
            UIPages::LocationGrid => {
                let cmd_id = Uuid::new_v4().to_string();
                self.async_tasks_to_send.push(CommandToServer::GetAllSlaves(
                    cmd_id.clone(),
                    self.source_node_id.clone(),
                    Vec::new(),
                ));
                self.functions_waiting_data.push(WaitingFunction {
                    id: cmd_id,
                    kind: WaitingFunctionKind::LoadLocationsPage,
                });
            }
            UIPages::LocationContainers => {
                self.async_tasks_to_send.push(CommandToServer::GetAllSlaves(
                    Uuid::new_v4().to_string(),
                    self.selected_location.id.clone(),
                    Vec::new(),
                ));
                self.search_string = String::new();
                self.current_ui = UIPages::LocationContainers;
            }
            UIPages::Statistics => {
                self.async_tasks_to_send.push(CommandToServer::GetAllSlaves(
                    Uuid::new_v4().to_string(),
                    self.source_node_id.clone(),
                    Vec::new(),
                ));
                self.async_tasks_to_send.push(CommandToServer::SearchItems(
                    Uuid::new_v4().to_string(),
                    "".to_string(),
                    "name".to_string(),
                    Vec::new(),
                ));
                self.calculate_statistics();

                self.current_ui = UIPages::Statistics;
            }
            UIPages::Account => {
                self.current_ui = UIPages::Account;
            }
        }
    }

    pub fn sort_locations(&mut self, order: &Vec<String>) {
        //Sorts the container_vec based on the order in ordered_locations_vec
        let mut result = Vec::new();
        for location_id in order {
            for location in &self.container_vec {
                if location.id == *location_id {
                    result.push(location.clone());
                    break;
                }
            }
        }
        //If no id is missing, replace the vec
        if self.container_vec.len() == result.len() {
            self.container_vec = result;
        }
    }

    pub fn themed_heading(ui: &mut egui::Ui, light_mode: bool, text: &str) -> Response {
        let resp: Response = if light_mode {
            ui.label(egui::RichText::new(text).heading().color(Color32::BLACK))
        } else {
            ui.label(
                egui::RichText::new(text)
                    .heading()
                    .color(Color32::LIGHT_GRAY),
            )
        };
        return resp;
    }

    pub fn execute_waiting_functions(&mut self) {
        let mut functions_executed: Vec<usize> = Vec::new();
        for (index, func_call) in self.functions_waiting_data.clone().into_iter().enumerate() {
            //If the id is no longer in the hashset, meaning the data has arrived, execute the function
            if !self.async_tasks_sent_ids.contains(&func_call.id) {
                match func_call.kind {
                    WaitingFunctionKind::LoadLocationsPage => {
                        if self.ordered_locations_vec.is_empty() && !self.container_vec.is_empty() {
                            //If there is no order in memory, use the order of the database
                            self.ordered_locations_vec =
                                self.container_vec.iter().map(|x| x.id.clone()).collect();
                        } else {
                            self.sort_locations(&self.ordered_locations_vec.clone());
                        }
                        self.rearrange_locations = false;
                        self.current_ui = UIPages::LocationGrid;
                    }
                    WaitingFunctionKind::AddFieldOk => {
                        self.async_tasks_to_send
                            .push(CommandToServer::GetItemColumnTypes(
                                Uuid::new_v4().to_string(),
                                Vec::new(),
                            ));
                        self.item_fields_shown.push(true);
                        self.modal_vars.field_modal = FieldModal::Start;
                        self.prepare_page(UIPages::Home);
                    }
                    WaitingFunctionKind::DeleteFieldOk => {
                        self.modal_vars.field_modal = FieldModal::Start;
                        self.prepare_page(UIPages::Home);
                    }
                    WaitingFunctionKind::DeleteContainerOk1 => {
                        self.selected_container = Container::default();
                        let cmd_id = Uuid::new_v4().to_string();
                        self.async_tasks_to_send.push(CommandToServer::GetAllSlaves(
                            cmd_id.clone(),
                            self.selected_location.id.clone(),
                            Vec::new(),
                        ));

                        self.functions_waiting_data.push(WaitingFunction {
                            id: cmd_id,
                            kind: WaitingFunctionKind::DeleteContainerOk2,
                        });
                    }
                    WaitingFunctionKind::DeleteContainerOk2 => {
                        self.container_vec.sort_by(|a, b| a.name.cmp(&b.name));
                        self.container_screen = ContainerScreen::None;
                    }
                    WaitingFunctionKind::AddExistingItemClicked1 => {
                        let cmd_id = Uuid::new_v4().to_string();
                        self.async_tasks_to_send
                            .push(CommandToServer::GetMultipleItems(
                                cmd_id.clone(),
                                self.containerless_items_ids.clone(),
                                Vec::new(),
                            ));
                        self.functions_waiting_data.push(WaitingFunction {
                            id: cmd_id,
                            kind: WaitingFunctionKind::AddExistingItemClicked2,
                        });
                    }
                    WaitingFunctionKind::AddExistingItemClicked2 => {
                        //The previous calls overwrite the item_vec with containerless_items
                        //Copy them over and call send another command to get the actual containers items
                        self.containerless_items = self.item_vec.clone();
                        self.containerless_items_bools =
                            vec![false; self.containerless_items.len()];
                        self.async_tasks_to_send
                            .push(CommandToServer::GetMultipleItems(
                                Uuid::new_v4().to_string(),
                                self.selected_container.contained_items.clone(),
                                Vec::new(),
                            ));
                        self.modal_vars.modal_type = ModalType::SelectContainerlessItem;
                    }
                }
                functions_executed.push(index);
            }
        }
        for idx in functions_executed.iter().rev() {
            self.functions_waiting_data.remove(*idx);
        }
    }
}
