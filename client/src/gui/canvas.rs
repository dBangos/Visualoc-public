use crate::{CommandToServer, ContainerScreen, Visualoc, database::data_helpers::ImageSize};
use egui::{Color32, FontFamily, Frame, Painter, Pos2, Rect, Response, Sense, Vec2};
use uuid::Uuid;

impl Visualoc {
    fn percent_to_screen(&self, x: &f32, y: &f32, response: &Response, image_size: &Vec2) -> Pos2 {
        //response.rect.min is there to offset the frame (from the top left corner of the window, to the top left corner of the canvas)
        //(response.rect.max.x-response.rect.min.x-image_size.x)/2 is there to center the image in the canvas
        return Pos2 {
            x: (response.rect.max.x - response.rect.min.x - image_size.x) / 2.0
                + response.rect.min.x
                + x * image_size.x,
            y: (response.rect.max.y - response.rect.min.y - image_size.y) / 2.0
                + response.rect.min.y
                + y * image_size.y,
        };
    }

    fn draw_all_slave_containers(
        &mut self,
        ui: &mut egui::Ui,
        painter: &mut Painter,
        response: &Response,
        image_size: &Vec2,
    ) {
        for container in &self.container_vec {
            let rect = Rect::from_two_pos(
                self.percent_to_screen(
                    &container.corners[0],
                    &container.corners[1],
                    response,
                    image_size,
                ),
                self.percent_to_screen(
                    &container.corners[2],
                    &container.corners[3],
                    response,
                    image_size,
                ),
            );
            let local_response = ui.interact(
                rect,
                container.id.clone().into(),
                Sense::click() | Sense::hover(),
            );
            if local_response.hovered() {
                painter.rect_stroke(
                    rect,
                    0.0,
                    (2.0, self.settings.border_colour),
                    egui::StrokeKind::Outside,
                );
            }
            if local_response.clicked() {
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
            painter.rect_filled(rect, 0.0, self.settings.rectangle_colour);
            //Paint the container name
            if self.settings.show_container_names && rect.height() > 0.0 && rect.width() > 0.0 {
                let mut painted_text_size = 100.0;
                let mut galley = painter.layout_no_wrap(
                    container.name.clone(),
                    egui::FontId::new(painted_text_size, FontFamily::Proportional),
                    self.settings.container_name_colour,
                );
                while galley.rect.width() > rect.width() || galley.rect.height() > rect.height() {
                    galley = painter.layout_no_wrap(
                        container.name.clone(),
                        egui::FontId::new(painted_text_size, FontFamily::Proportional),
                        self.settings.container_name_colour,
                    );
                    painted_text_size *= 0.95;
                }
                painter.galley(
                    rect.center() - (galley.rect.max - galley.rect.min) / 2.0,
                    galley,
                    self.settings.container_name_colour,
                );
            }
        }
    }

    pub fn draw_canvas(&mut self, ui: &mut egui::Ui) {
        Frame::canvas(ui.style()).show(ui, |ui| {
            let painter_size = Vec2 {
                x: ui.available_size().x * 2.0 / 3.0,
                y: ui.available_size().y,
            };
            let (mut response, mut painter) =
                ui.allocate_painter(painter_size, Sense::click_and_drag() | Sense::hover());
            painter.set_opacity(self.settings.rectangle_opacity);
            ui.set_clip_rect(response.rect);
            //***********************************
            //Paint the image
            //***********************************
            if self.redraw_canvas_image {
                //If the image was changed, remove the old one from memory
                self.loaded_images.remove(
                    &(self.selected_location.id.to_string()
                        + "."
                        + &self.selected_location.image_type),
                );
                self.redraw_canvas_image = false;
            }
            let image_name =
                &(self.selected_location.id.clone() + "." + &self.selected_location.image_type);
            //When if let chains are available this can be simplified
            if self.loaded_images.contains_key(image_name) {
                if let Some((texture, image_size)) = self.loaded_images[image_name].clone() {
                    if image_size == ImageSize::Large {
                        let image = egui::Image::new(&texture);
                        let mut image_size: Vec2;
                        if let Some(size) =
                            egui::Image::load_and_calc_size(&image, ui, painter_size)
                        {
                            image_size = size;
                        } else {
                            image_size = (200.0, 200.0).into();
                        }
                        //Scale the image to fit the canvas (Scaling from load_and_calc size happens automaticly in native, but needs to be done explicitly in WASM)
                        let canvas_size = response.rect.max - response.rect.min;
                        let image_ratio = image_size.x / image_size.y;
                        if image_ratio > canvas_size.x / canvas_size.y {
                            //If the image is wider than the canvas, the image will have to be bound on x
                            image_size.x = canvas_size.x;
                            image_size.y = canvas_size.x / image_ratio;
                        } else {
                            //If the image is taller than the canvas, the image will have to be bound on y
                            image_size.y = canvas_size.y;
                            image_size.x = canvas_size.y * image_ratio;
                        }
                        //Offset to center the image in the canvas
                        let offset = if response.rect.max.x - response.rect.min.x - image_size.x
                            > 0.1
                        {
                            Vec2 {
                                x: (response.rect.max.x - response.rect.min.x - image_size.x) / 2.0,
                                y: 0.0,
                            }
                        } else {
                            Vec2 {
                                x: 0.0,
                                y: (response.rect.max.y - response.rect.min.y - image_size.y) / 2.0,
                            }
                        };
                        let image_rect = Rect::from_two_pos(
                            response.rect.min + offset,
                            response.rect.min + offset + image_size,
                        );
                        image.paint_at(ui, image_rect);
                        //***********************************
                        //Paint all the rectangles of the containers
                        //***********************************
                        if self.settings.show_rectangles {
                            self.draw_all_slave_containers(
                                ui,
                                &mut painter,
                                &response,
                                &image_size,
                            );
                        }
                        //***********************************
                        //Paint selected container
                        //***********************************
                        if self.selected_container.id != String::new()
                            && self.settings.show_rectangles
                        {
                            painter.rect_filled(
                                Rect::from_two_pos(
                                    self.percent_to_screen(
                                        &self.selected_container.corners[0],
                                        &self.selected_container.corners[1],
                                        &response,
                                        &image_size,
                                    ),
                                    self.percent_to_screen(
                                        &self.selected_container.corners[2],
                                        &self.selected_container.corners[3],
                                        &response,
                                        &image_size,
                                    ),
                                ),
                                0.0,
                                self.settings.selected_rectangle_colour,
                            );
                        }
                        //***********************************
                        //Paint the rectangle to be added
                        //***********************************
                        if self.container_screen == ContainerScreen::AddingContainer
                            || self.container_screen == ContainerScreen::EditingContainer
                        {
                            //New response to contain the initial interaction within the image_rect
                            response = ui.interact(
                                image_rect,
                                "image_rect".into(),
                                Sense::click_and_drag(),
                            );
                            if response.dragged() {
                                self.selected_container.corners[2] +=
                                    response.drag_delta().x / image_size.x;
                                self.selected_container.corners[3] +=
                                    response.drag_delta().y / image_size.y;
                                //Constraining the second point to be inside the image
                                self.selected_container.corners[2] =
                                    self.selected_container.corners[2].clamp(0.0, 1.0);
                                self.selected_container.corners[3] =
                                    self.selected_container.corners[3].clamp(0.0, 1.0);
                            } else if response.drag_started()
                                || response.clicked()
                                || response.is_pointer_button_down_on()
                            {
                                //This runs a few extra times when the item is not being actively dragged
                                //But it feels more responsive
                                if let Some(point) = response.interact_pointer_pos {
                                    //Initial point
                                    (
                                        self.selected_container.corners[0],
                                        self.selected_container.corners[1],
                                    ) = point.into();
                                }
                                self.selected_container.corners[2] =
                                    self.selected_container.corners[0];
                                self.selected_container.corners[3] =
                                    self.selected_container.corners[1];
                                //Offset from the window border
                                self.selected_container.corners[0] -= response.rect.min.x;
                                self.selected_container.corners[1] -= response.rect.min.y;
                                self.selected_container.corners[2] -= response.rect.min.x;
                                self.selected_container.corners[3] -= response.rect.min.y;
                                //Scale to image percentage
                                self.selected_container.corners[0] /= image_size.x;
                                self.selected_container.corners[1] /= image_size.y;
                                self.selected_container.corners[2] /= image_size.x;
                                self.selected_container.corners[3] /= image_size.y;
                            }
                            painter.rect_filled(
                                Rect::from_two_pos(
                                    self.percent_to_screen(
                                        &self.selected_container.corners[0],
                                        &self.selected_container.corners[1],
                                        &response,
                                        &image_size,
                                    ),
                                    self.percent_to_screen(
                                        &self.selected_container.corners[2],
                                        &self.selected_container.corners[3],
                                        &response,
                                        &image_size,
                                    ),
                                ),
                                0.0,
                                Color32::WHITE,
                            );
                        }
                    } else {
                        self.loaded_images.insert(image_name.to_owned(), None);
                        self.async_tasks_to_send
                            .push(CommandToServer::GetImageFromServer(
                                Uuid::new_v4().to_string(),
                                self.selected_location.id.clone(),
                                self.selected_location.image_type.clone(),
                                crate::database::data_helpers::ImageSize::Large,
                                egui::ColorImage::default(),
                            ));
                    }
                }
            } else {
                self.loaded_images.insert(image_name.to_owned(), None);
                self.async_tasks_to_send
                    .push(CommandToServer::GetImageFromServer(
                        Uuid::new_v4().to_string(),
                        self.selected_location.id.clone(),
                        self.selected_location.image_type.clone(),
                        crate::database::data_helpers::ImageSize::Large,
                        egui::ColorImage::default(),
                    ));
            }
        });
    }
}
