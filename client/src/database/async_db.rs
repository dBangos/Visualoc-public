//State holds a run_time, a channel and 2 vec. One vec is used to send the tasks, the other to keep their order.
//Each frame any tasks accumulated get sent, and any data received gets parsed
//Each command has its own id that gets stored in a hashmap when it is sent. It is then checked on arrival and removed
//The existence of said id can be used to wait for the data, using the WaitingFunction struct and the execute_waiting_functions fucntion
//that gets run on every frame.

use log::{Level, log};
use reqwest::Client;
use std::collections::HashSet;
use tokio::sync::mpsc::{self, Sender};
use uuid::Uuid;
use wasm_bindgen_futures::spawn_local;

use crate::{
    CommandToServer, Container, ContainerScreen, DataType, LoginResult, UIPages, Visualoc,
    gui::login::{initial_authentication, login_user_request, register_user_request},
};

use super::{
    containers::{add_container, delete_container, get_all_slaves, update_container},
    data_helpers::{
        add_column_to_items, add_image, delete_column_from_items,
        get_all_item_ids_not_in_container, get_backup_from_server, get_image_from_server,
        pick_dump_file, pick_image_folder, update_items_column, upload_backup,
    },
    items::{
        delete_item, get_item_location_container, get_multiple_items, insert_item, search_items,
        update_item,
    },
};

pub fn get_item_column_types(host: &str, tx: &Sender<CommandToServer>, id: &str, token: &str) {
    let id = id.to_owned();
    let host = host.to_owned();
    let tx = tx.clone();
    let token = token.to_owned();

    spawn_local(async move {
        let response = Client::new()
            .post(host + "get_item_column_types")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;

        match response {
            Ok(resp) => match resp.json::<Vec<(String, DataType)>>().await {
                Ok(vec) => {
                    if let Err(e) = tx.send(CommandToServer::GetItemColumnTypes(id, vec)).await {
                        println!("Error when sending the item_column_types back: {}", e);
                    }
                }
                Err(e) => println!("Error while deserializing json item columns: {}", e),
            },
            Err(e) => println!("Gett item column types error {}", e),
        }
    });
}

pub fn send_command_to_database(
    sender: &Sender<CommandToServer>,
    vec_cmd: &mut Vec<CommandToServer>,
    id_vec: &mut HashSet<String>,
    host: &str,
    token: &str,
) {
    for cmd in &mut *vec_cmd {
        log!(Level::Info, "Sending request: {:?}", cmd);
        match cmd {
            CommandToServer::GetItemColumnTypes(id, _) => {
                id_vec.insert(id.clone());
                get_item_column_types(host, sender, id, token);
            }
            CommandToServer::SearchItems(id, value, column_name, _) => {
                id_vec.insert(id.clone());
                search_items(host, sender, id, token, value, column_name);
            }
            CommandToServer::AddContainer(id, container) => {
                id_vec.insert(id.clone());
                add_container(host, sender, id, token, container);
            }
            CommandToServer::GetAllSlaves(id, source_id, _) => {
                id_vec.insert(id.clone());
                get_all_slaves(host, sender, id, token, source_id);
            }
            CommandToServer::InsertItem(id, item, container_id, field_vec) => {
                id_vec.insert(id.clone());
                insert_item(host, sender, id, token, item, container_id, field_vec);
            }
            CommandToServer::UpdateItem(id, item, field_vec) => {
                id_vec.insert(id.clone());
                update_item(host, sender, id, token, item, field_vec);
            }
            CommandToServer::AddField(id, name, data_type) => {
                id_vec.insert(id.clone());
                add_column_to_items(host, sender, id, token, (name.clone(), data_type.clone()))
            }
            CommandToServer::GetItemLocationContainer(id, item_id, _) => {
                id_vec.insert(id.clone());
                get_item_location_container(host, sender, id, token, item_id);
            }
            CommandToServer::DeleteColumnFromItems(id, column_name) => {
                id_vec.insert(id.clone());
                delete_column_from_items(host, sender, id, token, column_name);
            }
            CommandToServer::UpdateItemsColumn(id, old, new) => {
                id_vec.insert(id.clone());
                update_items_column(host, sender, id, token, old, new);
            }
            CommandToServer::DeleteContainer(id, container) => {
                id_vec.insert(id.clone());
                delete_container(host, sender, id, token, container);
            }
            CommandToServer::UpdateContainer(id, container) => {
                id_vec.insert(id.clone());
                update_container(host, sender, id, token, container);
            }
            CommandToServer::DeleteItem(id, item, container, delete_from_database) => {
                id_vec.insert(id.clone());
                delete_item(
                    host,
                    sender,
                    id,
                    token,
                    item,
                    container,
                    delete_from_database,
                );
            }
            CommandToServer::GetMultipleItems(id, item_ids, _) => {
                id_vec.insert(id.clone());
                get_multiple_items(host, sender, id, token, item_ids);
            }
            CommandToServer::GetAllItemIdsNotInContainer(id, _) => {
                id_vec.insert(id.clone());
                get_all_item_ids_not_in_container(host, sender, id, token);
            }
            CommandToServer::RegisterUser(id, _, username, password, email) => {
                id_vec.insert(id.clone());
                register_user_request(host, sender, id, username, password, email);
            }
            CommandToServer::LoginUser(id, _, username, password) => {
                id_vec.insert(id.clone());
                login_user_request(host, sender, id, username, password);
            }
            CommandToServer::GetImageFromServer(id, image_id, image_type, image_size, _) => {
                id_vec.insert(id.clone());
                get_image_from_server(host, sender, id, token, image_id, image_type, image_size);
            }
            CommandToServer::AddImage(cmd_id, item_id, _) => {
                id_vec.insert(cmd_id.clone());
                add_image(host, sender, cmd_id, token, item_id);
            }
            CommandToServer::GetBackup(cmd_id) => {
                id_vec.insert(cmd_id.clone());
                get_backup_from_server(host, sender, cmd_id, token);
            }
            CommandToServer::PickBackupDumpFile(cmd_id, _) => {
                id_vec.insert(cmd_id.clone());
                pick_dump_file(sender, cmd_id);
            }
            CommandToServer::PickBackupImageFolder(cmd_id, _) => {
                id_vec.insert(cmd_id.clone());
                pick_image_folder(sender, cmd_id);
            }
            CommandToServer::UploadBackup(cmd_id, file_handle, file_list) => {
                if let Some(dump_file) = file_handle {
                    if let Some(image_folder) = file_list {
                        id_vec.insert(cmd_id.clone());
                        upload_backup(host, sender, cmd_id, token, dump_file, image_folder);
                    }
                }
            }
            CommandToServer::Authenticate(cmd_id, persistent_token, _) => {
                id_vec.insert(cmd_id.clone());
                initial_authentication(host, sender, cmd_id, persistent_token);
            }
        }
    }
    *vec_cmd = Vec::new();
}

impl Visualoc {
    pub fn parse_command(&mut self, ctx: &egui::Context) {
        match self.tokio_receiver.try_recv() {
            Ok(command) => match &command {
                CommandToServer::GetItemColumnTypes(id, vec) => {
                    for column in vec {
                        log!(Level::Info, "Name: {}, Type: {:?}", column.0, column.1);
                    }
                    self.async_tasks_sent_ids.remove(id);
                    self.item_field_types = vec.clone();
                    println!("Made it to parse_async_tasks get_item_column_types")
                }
                CommandToServer::SearchItems(id, _, _, vec) => {
                    self.async_tasks_sent_ids.remove(id);
                    println!("Made it to parse_async_tasks search_items");
                    self.item_vec = vec.clone();
                    self.item_vec.sort_by(|a, b| a.name.cmp(&b.name));
                }
                CommandToServer::AddContainer(id, _) => {
                    println!("Made it to parse_async_tasks add_container");
                    self.async_tasks_sent_ids.remove(id);
                }
                CommandToServer::GetAllSlaves(id, _, containers) => {
                    self.async_tasks_sent_ids.remove(id);
                    self.container_vec = containers.clone();
                    println!("Made it to parse_async_tasks get_all_first_level_slaves")
                }
                CommandToServer::InsertItem(id, _, _, _) => {
                    self.async_tasks_sent_ids.remove(id);
                    println!("Made it to parse_async_tasks insert_or_update_item")
                }
                CommandToServer::UpdateItem(id, _, _) => {
                    self.async_tasks_sent_ids.remove(id);
                    println!("Made it to parse_async_tasks update_item")
                }
                CommandToServer::AddField(id, _, _) => {
                    self.async_tasks_sent_ids.remove(id);
                    println!("Made it to parse_async_tasks add_field")
                }
                CommandToServer::GetItemLocationContainer(id, _, cont_loc) => {
                    self.async_tasks_sent_ids.remove(id);
                    match cont_loc {
                        Some((cont, loc)) => {
                            self.selected_location = loc.clone();
                            self.selected_container = cont.clone();
                            self.container_screen = ContainerScreen::SelectedItem;
                            self.current_ui = UIPages::LocationContainers;
                        }
                        None => {
                            self.selected_location = Container::default();
                            self.selected_container = Container::default();
                            self.container_screen = ContainerScreen::ItemNotInContainer;
                            self.current_ui = UIPages::LocationContainers;
                            self.container_vec = Vec::new();
                        }
                    }
                    self.async_tasks_to_send.push(CommandToServer::GetAllSlaves(
                        Uuid::new_v4().to_string(),
                        self.selected_location.id.clone(),
                        Vec::new(),
                    ));
                    println!("Made it to parse_async_tasks get_item_location_container")
                }
                CommandToServer::DeleteColumnFromItems(id, _) => {
                    self.async_tasks_sent_ids.remove(id);
                    println!("Made it to parse_async_tasks delete_column")
                }
                CommandToServer::UpdateItemsColumn(id, _, _) => {
                    self.async_tasks_sent_ids.remove(id);
                    println!("Made it to parse_async_tasks rename_column")
                }
                CommandToServer::DeleteContainer(id, _) => {
                    println!("Made it to parse_async_tasks delete_container");
                    self.async_tasks_sent_ids.remove(id);
                }
                CommandToServer::UpdateContainer(id, _) => {
                    println!("Made it to parse_async_tasks update_container");
                    self.async_tasks_sent_ids.remove(id);
                }
                CommandToServer::DeleteItem(id, _, _, _) => {
                    self.async_tasks_sent_ids.remove(id);
                    println!("Made it to parse_async_tasks delete_item")
                }
                CommandToServer::GetMultipleItems(id, _, vec) => {
                    self.async_tasks_sent_ids.remove(id);
                    self.item_vec = vec.clone();
                    self.item_vec.sort_by(|a, b| a.name.cmp(&b.name));
                    println!("Made it to parse_async_tasks get_multiple_items")
                }
                CommandToServer::GetAllItemIdsNotInContainer(id, vec) => {
                    self.async_tasks_sent_ids.remove(id);
                    self.containerless_items_ids = vec.clone().into_iter().collect();
                    println!("Made it to parse_async_tasks get_all_item_ids_not_in_container")
                }
                CommandToServer::RegisterUser(id, register_result, _, _, _) => {
                    self.async_tasks_sent_ids.remove(id);
                    self.login.register_result = *register_result;
                    println!("Made it to parse_async_tasks register_user")
                }
                CommandToServer::LoginUser(id, login_result, _, _) => {
                    self.async_tasks_sent_ids.remove(id);
                    self.login.login_result = login_result.clone();
                }
                CommandToServer::GetImageFromServer(
                    id,
                    image_id,
                    image_type,
                    image_size,
                    color_image,
                ) => {
                    self.async_tasks_sent_ids.remove(id);
                    let image_name = image_id.to_owned() + "." + image_type;
                    let texture = ctx.load_texture(
                        &image_name,
                        color_image.to_owned(),
                        egui::TextureOptions::default(),
                    );
                    self.loaded_images
                        .insert(image_name, Some((texture, image_size.clone())));
                }
                CommandToServer::AddImage(cmd_id, item_id, image_type) => {
                    self.async_tasks_sent_ids.remove(cmd_id);
                    if self.selected_item.id == *item_id {
                        self.selected_item.image_type = image_type.to_string();
                    } else if self.selected_container.id == *item_id {
                        self.selected_container.image_type = image_type.to_string();
                    } else if self.selected_location.id == *item_id {
                        self.selected_location.image_type = image_type.to_string();
                    }
                }
                CommandToServer::GetBackup(cmd_id) => {
                    self.async_tasks_sent_ids.remove(cmd_id);
                }
                CommandToServer::PickBackupDumpFile(cmd_id, filehandle) => {
                    self.async_tasks_sent_ids.remove(cmd_id);
                    self.backup.dump_filehandle = filehandle.clone();
                }
                CommandToServer::PickBackupImageFolder(cmd_id, filelist) => {
                    self.async_tasks_sent_ids.remove(cmd_id);
                    self.backup.images_filehandle = filelist.clone();
                }
                CommandToServer::UploadBackup(cmd_id, _, _) => {
                    self.async_tasks_sent_ids.remove(cmd_id);
                }
                CommandToServer::Authenticate(cmd_id, _, login_result) => {
                    self.async_tasks_sent_ids.remove(cmd_id);
                    match login_result {
                        LoginResult::Success(val) => {
                            self.login.session_token = val.clone();
                            self.persistent_token = val.clone();
                        }
                        _ => {
                            self.remember_login = false;
                            self.persistent_token = String::new();
                        }
                    }
                }
            },
            Err(e) => match e {
                mpsc::error::TryRecvError::Empty => (),
                mpsc::error::TryRecvError::Disconnected => println!("Disconnected"),
            },
        }
    }
}
