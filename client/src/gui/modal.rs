use crate::{
    BackupState, CommandToServer, ContainerScreen, ModalType, UIPages, Visualoc, WaitingFunction,
    WaitingFunctionKind, database::data_helpers::ImageSize, toggle_light_mode,
};
use egui::{Color32, ColorImage, Label, Layout, Modal, Slider};
use uuid::Uuid;

impl Visualoc {
    pub fn spawn_modal(&mut self, ctx: &egui::Context, modal_id: String) {
        Modal::new(modal_id.into()).show(ctx, |ui| {
            match self.modal_vars.modal_type {
                ModalType::None => (),
                ModalType::DeleteLocation => {
                    ui.label("Confirm Deletion");
                    ui.add_space(10.0);
                    ui.add(Label::new("Are you sure you want to permanently delete this location?").wrap());
                }
                ModalType::DeleteContainer => {
                    ui.label("Confirm Deletion");
                    ui.add_space(10.0);
                    ui.add(Label::new("Are you sure you want to permanently delete this container? This will not delete the items inside this container.").wrap());
                }
                ModalType::DeleteItem => {
                    ui.label("Confirm Deletion");
                    ui.add_space(10.0);
                    ui.add(Label::new("Are you sure you want to permanently delete this item?").wrap());
                }
                ModalType::DeleteField => {
                    ui.label("Confirm Deletion");
                    ui.add_space(10.0);
                    ui.add(Label::new("Are you sure you want to permanently delete this field? This will also delete the fields value for each item.").wrap());
                }
                ModalType::RemoveFromContainer=>{
                    ui.label("Confirm Deletion");
                    ui.add_space(10.0);
                    ui.add(Label::new("This will remove the item from this container but the item will remain in the database. To add the item to another container, navigate to that container and select 'Add Existing Item'").wrap());
                }
                ModalType::SelectContainerlessItem=>{
                    if self.containerless_items.is_empty(){
                        ui.add(Label::new("There are no existing items not in a container.").wrap());
                    }else{
                        ui.add(Label::new("Select the items to add to this container.").wrap());
                        for (index,item) in self.containerless_items.iter().enumerate(){
                            ui.checkbox(&mut self.containerless_items_bools[index], &item.name);
                        }
                    }
                }
                ModalType::SelectFieldsShown=>{
                    ui.add(Label::new("Select which fields will be shown.").wrap());
                    for (index, field) in self.item_field_types.iter().enumerate(){
                        ui.checkbox(&mut self.item_fields_shown[index], field.0.clone());
                    }
                }
                ModalType::ItemImage=>{
                    //When if let can be integrated into chains this code can be simplified
                    let image_name=&(self.selected_item.id.clone()+"."+&self.selected_item.image_type);
                    if self.loaded_images.contains_key(image_name){
                        if let Some((texture, size))= self.loaded_images[image_name].clone(){
                            if size==ImageSize::Large{
                                ui.add(
                                    egui::Image::new(&texture)
                                        .fit_to_original_size(2.0)
                                        .max_height(500.0)
                                        .sense(egui::Sense::click()),
                                );
                            }else{
                                self.loaded_images.insert(image_name.to_owned(), None);
                                self.async_tasks_to_send.push(CommandToServer::GetImageFromServer(Uuid::new_v4().to_string(),
                                    self.selected_item.id.clone(),
                                    self.selected_item.image_type.clone(),
                                    crate::database::data_helpers::ImageSize::Large,
                                    ColorImage::default()
                                ));
                            }
                        }
                    }else{
                        self.loaded_images.insert(image_name.to_owned(), None);
                        self.async_tasks_to_send.push(CommandToServer::GetImageFromServer(Uuid::new_v4().to_string(),
                            self.selected_item.id.clone(),
                            self.selected_item.image_type.clone(),
                            crate::database::data_helpers::ImageSize::Large,
                            ColorImage::default()
                        ));
                    }
                }
                ModalType::Backup=>{
                    match self.backup.state{
                        BackupState::Start=>{
                            Visualoc::themed_heading(ui, self.settings.light_mode, "Download Backup");
                            ui.add(Label::new("This will download a copy of your data.").wrap());
                            if ui.button("Download Backup").clicked(){
                                self.async_tasks_to_send.push(CommandToServer::GetBackup(Uuid::new_v4().to_string()));
                            }
                            ui.add_space(20.0);
                            ui.separator();
                            Visualoc::themed_heading(ui, self.settings.light_mode, "Restore from Backup");
                            ui.add(Label::new("You can upload a backup to restore your data to that previous state").wrap());
                            if ui.button("Upload backup").clicked(){
                                self.backup.state=BackupState::Upload;
                            }
                        }
                        BackupState::Upload=>{
                            ui.add(Label::new("Uploading a backup will overwrite the current data. It is recommended to download a backup of the current state before reverting to an older one.").wrap());
                            if ui.button("Download Backup").clicked(){
                                self.async_tasks_to_send.push(CommandToServer::GetBackup(Uuid::new_v4().to_string()));
                            }
                            ui.separator();
                            ui.add_space(20.0);
                            ui.add(Label::new("Select the dump file you want to restore from:").wrap());
                            if self.backup.dump_filehandle.is_none(){
                                ui.colored_label(Color32::RED, "Please select a valid sql dump file");
                            }
                            if ui.button("Upload Dump File").clicked(){
                                self.async_tasks_to_send.push(CommandToServer::PickBackupDumpFile(Uuid::new_v4().to_string(), None));
                            }
                            ui.add(Label::new("Select the images folder you want to restore from:").wrap());
                            if self.backup.images_filehandle.is_none(){
                                ui.colored_label(Color32::RED, "Please select a valid images folder");
                            }
                            if ui.button("Upload Images Folder").clicked(){
                                self.async_tasks_to_send.push(CommandToServer::PickBackupImageFolder(Uuid::new_v4().to_string(), None));
                            }
                            ui.add(Label::new("Once you have selected both the dump file and the images folder click ok to start uploading the files").wrap());
                        }
                        BackupState::Waiting=>{
                            ui.label(&self.backup.resulting_string);
                        }
                    }
                    ui.add_space(35.0);
                }
                ModalType::Settings=>{
                    Visualoc::themed_heading(ui, self.settings.light_mode, "Settings");
                    ui.separator();
                    let lightmode_check=ui.checkbox(&mut self.settings.light_mode, "Light Mode");
                    if lightmode_check.clicked(){
                        toggle_light_mode(ctx, self.settings.light_mode);
                    }
                    ui.horizontal(|ui|{
                        ui.label("UI Scale:");
                    ui.add(Slider::new(&mut self.settings.ui_scale_temp,0.5..=2.0).step_by(0.1));
                    if ui.button("Apply").clicked(){
                        self.settings.ui_scale = self.settings.ui_scale_temp;
                    }
                    });
                    ui.horizontal(|ui|{
                        ui.label("Items Per Page:");
                        let original_value = self.settings.items_per_page;
                        ui.add(egui::DragValue::new(&mut self.settings.items_per_page).range(1..=100));
                        //If the value changes, go to the first page to avoid showing nonexistant items
                        if self.settings.items_per_page!=original_value{
                            self.home_page.page_number=0;
                        }
                    });
                    ui.checkbox(&mut self.settings.show_rectangles, "Show Containers on Image");
                    ui.horizontal(|ui|{
                        ui.label("Container Opacity:");
                    ui.add(Slider::new(&mut self.settings.rectangle_opacity,0.0..=1.0));
                    });
                    ui.horizontal(|ui|{
                        ui.label("Container Colour:");
                        ui.color_edit_button_srgba(&mut self.settings.rectangle_colour);
                    });
                    ui.horizontal(|ui|{
                        ui.label("Selected Container Colour:");
                        ui.color_edit_button_srgba(&mut self.settings.selected_rectangle_colour);
                    });
                    ui.horizontal(|ui|{
                        ui.label("Border Colour:");
                        ui.color_edit_button_srgba(&mut self.settings.border_colour);
                    });
                    ui.checkbox(&mut self.settings.show_container_names, "Show Container Names On Image");
                    ui.horizontal(|ui|{
                        ui.label("Container Name Colour:");
                        ui.color_edit_button_srgba(&mut self.settings.container_name_colour);
                    });
                    ui.separator();
                },
                ModalType::AddLocation=>{
                    ui.vertical_centered(|ui|{
                        if self.selected_location.image_type != String::default() {
                            let image_name=&(self.selected_location.id.clone()+"."+&self.selected_location.image_type);
                            if self.loaded_images.contains_key(image_name){
                                if let Some((texture,_))= self.loaded_images[image_name].clone(){
                                    ui.add(
                                        egui::Image::new(&texture)
                                            .fit_to_original_size(2.0)
                                            .max_height(500.0)
                                            .sense(egui::Sense::click()),
                                    );
                                }
                            }else{
                                self.loaded_images.insert(image_name.to_owned(), None);
                                self.async_tasks_to_send.push(CommandToServer::GetImageFromServer(Uuid::new_v4().to_string(),
                                    self.selected_location.id.clone(),
                                    self.selected_location.image_type.clone(),
                                    crate::database::data_helpers::ImageSize::Medium,
                                    ColorImage::default()
                                ));
                            }
                        }
                        ui.add(egui::TextEdit::singleline(&mut self.selected_location.name).hint_text("Location Name"));
                        if ui.button("Add Image").clicked() {
                            self.loaded_images.remove(
                                &(self.selected_location.id.to_string()
                                    + "."
                                    + &self.selected_location.image_type),
                            );
                            self.async_tasks_to_send.push(CommandToServer::AddImage(Uuid::new_v4().to_string(), self.selected_location.id.clone(), String::new()));
                            self.modal_vars.modal_id=Uuid::new_v4().to_string();
                        }
                    });
                    ui.add_space(15.0);
                    ui.separator();
                }
            }
            ui.with_layout(Layout::left_to_right(egui::Align::Center),|ui|{
                ui.add_space(ui.available_width()/2.0-85.0);
                if Visualoc::ok_button(ui).clicked(){
                    match self.modal_vars.modal_type{
                        ModalType::None=>(),
                        ModalType::DeleteContainer=>{
                            let cmd_id = Uuid::new_v4().to_string();
                            self.async_tasks_to_send.push(CommandToServer::DeleteContainer(cmd_id.clone(),self.selected_container.clone()));
                            self.functions_waiting_data.push(WaitingFunction {
                                id: cmd_id,
                                kind: WaitingFunctionKind::DeleteContainerOk1,
                            });
                        }
                        ModalType::DeleteLocation=>{
                            self.async_tasks_to_send.push(CommandToServer::DeleteContainer(Uuid::new_v4().to_string(),self.selected_location.clone()));
                            self.ordered_locations_vec.retain(|x| *x!=self.selected_location.id);
                            self.prepare_page(UIPages::LocationGrid);
                        }
                        ModalType::DeleteItem=>{
                            self.async_tasks_to_send.push(CommandToServer::DeleteItem(Uuid::new_v4().to_string(),self.selected_item.clone(), self.selected_container.id.clone(), true));
                            if self.container_screen==ContainerScreen::ItemNotInContainer{
                                self.prepare_page(UIPages::Home);
                            }
                            else {
                                self.selected_container
                                    .contained_items
                                    .remove(&self.selected_item.id);
                                self.async_tasks_to_send
                                    .push(CommandToServer::GetMultipleItems(Uuid::new_v4().to_string(),
                                        self.selected_container.contained_items.clone(),
                                        Vec::new(),
                                    ));
                                self.search_string = "".into();
                                self.item_page_search_vec = self.item_vec.clone();
                                self.container_screen = ContainerScreen::SelectedContainer;
                            }
                        }
                        ModalType::DeleteField=>{
                            for (to_be_deleted, (column_name, _)) in self
                                .modal_vars.item_field_selected_fields
                                .iter()
                                .zip(&self.item_field_types)
                            {
                                if *to_be_deleted {
                                    self.async_tasks_to_send.push(CommandToServer::DeleteColumnFromItems(Uuid::new_v4().to_string(),column_name.clone()));
                                }
                            }
                            let cmd_id= Uuid::new_v4().to_string();
                            self.async_tasks_to_send
                                .push(CommandToServer::GetItemColumnTypes(cmd_id.clone(),Vec::new()));
                            self.functions_waiting_data.push(WaitingFunction { id: cmd_id, kind: WaitingFunctionKind::DeleteFieldOk });
                        }
                        ModalType::RemoveFromContainer=>{
                            self.async_tasks_to_send.push(CommandToServer::DeleteItem(Uuid::new_v4().to_string(),self.selected_item.clone(), self.selected_container.id.clone(), false));
                            self.selected_container
                                .contained_items
                                .remove(&self.selected_item.id);
                            self.async_tasks_to_send
                                .push(CommandToServer::GetMultipleItems(Uuid::new_v4().to_string(),
                                    self.selected_container.contained_items.clone(),
                                    Vec::new(),
                                ));
                            //Update the Container in the ContainerVec
                            for cont in &mut self.container_vec {
                                if cont.id == self.selected_container.id {
                                    cont.contained_items.remove(&self.selected_item.id.clone());
                                }
                            }
                            self.search_string = "".into();
                            self.item_page_search_vec = self.item_vec.clone();
                            self.container_screen = ContainerScreen::SelectedContainer;
                        }
                        ModalType::SelectContainerlessItem=>{
                            for (index,val) in self.containerless_items_bools.iter().enumerate(){
                                if *val{
                                    //Update the containers contained_items in memory
                                    self.selected_container.contained_items.insert(self.containerless_items[index].id.clone());
                                    //Update the containers contained_items on the database
                                    self.async_tasks_to_send.push(CommandToServer::UpdateContainer(Uuid::new_v4().to_string(),self.selected_container.clone()));
                                    //Update the Container in the ContainerVec
                                    for cont in &mut self.container_vec {
                                        if cont.id == self.selected_container.id {
                                            cont.contained_items.insert(self.containerless_items[index].id.clone());
                                        }
                                    }
                                    //Update the item vec
                                    self.item_vec.push(self.containerless_items[index].clone());
                                    //Update the search vec
                                    self.item_page_search_vec=self.item_vec.clone();
                                }
                            }
                        }
                        ModalType::SelectFieldsShown=>(),
                        ModalType::ItemImage=>(),
                        ModalType::Backup=>{
                            match self.backup.state{
                                BackupState::Start=>{

                                    self.modal_vars.modal_type=ModalType::None;
                                    self.modal_vars.modal_id=Uuid::new_v4().to_string();
                                }
                                BackupState::Upload=>{
                                    if self.backup.dump_filehandle.is_some() && self.backup.images_filehandle.is_some(){
                                        self.backup.state=BackupState::Waiting;
                                        self.async_tasks_to_send.push(CommandToServer::UploadBackup(Uuid::new_v4().to_string(), self.backup.dump_filehandle.clone(), self.backup.images_filehandle.clone()));
                                    }
                                }
                                BackupState::Waiting=>()
                            }
                        }
                        ModalType::Settings=>(),
                        ModalType::AddLocation=>{
                            self.async_tasks_to_send.push(CommandToServer::UpdateContainer(Uuid::new_v4().to_string(), self.selected_location.clone()));
                            self.prepare_page(UIPages::LocationGrid);
                        }
                    }
                    //Backup handles it's own state
                    if self.modal_vars.modal_type!=ModalType::Backup{
                        self.modal_vars.modal_type=ModalType::None;
                        self.modal_vars.modal_id=Uuid::new_v4().to_string();
                    }

                }
                if Visualoc::cancel_button(ui).clicked(){
                    //If the user cancels out of the settings, discard all the changes
                    if self.modal_vars.modal_type==ModalType::Settings{
                        self.settings=self.temp_settings.clone();
                    }
                    if self.modal_vars.modal_type==ModalType::AddLocation{
                        self.async_tasks_to_send.push(CommandToServer::DeleteContainer(Uuid::new_v4().to_string(), self.selected_location.clone()));
                    }
                    self.modal_vars.modal_type=ModalType::None;
                    self.modal_vars.modal_id=Uuid::new_v4().to_string();
                }
            });
        });
    }
}
