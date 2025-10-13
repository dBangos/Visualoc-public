// How the dynamic fields work:
//     Struct side/Rust side:
//         Each item has 3 vectors, one for each type of data
//         There is a global vector with the names of the new fields, their types and their order
//         The data gets stored on the proper vector based on its type
//         The first in the i64 list corresponds to the first i64 name on the global name list and so on

//     Database side/Sqlite side:
//         The items table get a new column for each variable
//         These variables get parsed into the rust struct vectors

use std::collections::BTreeSet;

use reqwest::Client;
use serde::Serialize;
use tokio::sync::mpsc::Sender;
use wasm_bindgen_futures::spawn_local;

use crate::{CommandToServer, ContainedItem, Container, DataType};

// Struct for the request payload (matches MultipleItemRequest on the server)
#[derive(Serialize)]
struct IdVectorRequest {
    id_vec: Vec<String>,
}

#[derive(Serialize)]
struct SearchItemsRequest {
    search_string: String,
    column_name: String,
}

#[derive(Serialize)]
struct IdRequest {
    id: String,
}

#[derive(Serialize)]
struct InsertItemRequest {
    container_id: String,
    item: ContainedItem,
    item_columns_names_types: Vec<(String, DataType)>,
}

#[derive(Serialize)]
struct DeleteItemRequest {
    container_id: String,
    item: ContainedItem,
    delete_from_items: bool,
}

pub fn get_multiple_items(
    host: &str,
    tx: &Sender<CommandToServer>,
    id: &str,
    token: &str,
    item_ids: &BTreeSet<String>,
) {
    let id = id.to_owned();
    let host = host.to_owned();
    let id_vec: Vec<String> = item_ids.clone().into_iter().collect();
    let tx = tx.clone();
    let token = token.to_owned();
    spawn_local(async move {
        let request_data = IdVectorRequest { id_vec };
        let response = Client::new()
            .post(host + "get_multiple_items")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .send()
            .await;

        match response {
            Ok(resp) => match resp.json().await {
                Ok(vec) => {
                    if let Err(e) = tx
                        .send(CommandToServer::GetMultipleItems(id, BTreeSet::new(), vec))
                        .await
                    {
                        println!("Error when sending multiple items back: {}", e);
                    }
                }
                Err(e) => println!("Error while deserializing json get_multiple_items: {}", e),
            },
            Err(e) => println!("Get multiple items error {}", e),
        }
    });
}

pub fn search_items(
    host: &str,
    tx: &Sender<CommandToServer>,
    id: &str,
    token: &str,
    value: &str,
    column_name: &str,
) {
    let id = id.to_owned();
    let host = host.to_owned();
    let tx = tx.clone();
    let value = value.to_owned();
    let token = token.to_owned();
    let column_name = column_name.to_owned();
    spawn_local(async move {
        let request_data = SearchItemsRequest {
            column_name: column_name.clone(),
            search_string: value.clone(),
        };
        let response = Client::new()
            .post(host + "search_items")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .send()
            .await;

        match response {
            Ok(resp) => {
                println!("{:?}", resp);
                match resp.json().await {
                    Ok(vec) => {
                        if let Err(e) = tx
                            .send(CommandToServer::SearchItems(id, value, column_name, vec))
                            .await
                        {
                            println!("Error when sending search items back: {}", e);
                        }
                    }
                    Err(e) => println!("Error while deserializing json search_items: {}", e),
                }
            }
            Err(e) => println!("Search items error {}", e),
        }
    });
}

pub fn get_item_location_container(
    host: &str,
    tx: &Sender<CommandToServer>,
    cmd_id: &str,
    token: &str,
    item_id: &str,
) {
    let host = host.to_owned();
    let cmd_id = cmd_id.to_owned();
    let item_id = item_id.to_owned();
    let token = token.to_owned();
    let tx = tx.clone();
    spawn_local(async move {
        let request_data = IdRequest { id: item_id };
        let response = Client::new()
            .post(host + "get_item_container_location")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .send()
            .await;

        match response {
            Ok(resp) => match resp.json::<Option<(Container, Container)>>().await {
                Ok(cont1) => match cont1.clone() {
                    Some((container, location)) => {
                        if let Err(e) = tx
                            .send(CommandToServer::GetItemLocationContainer(
                                cmd_id,
                                String::default(),
                                Some((container, location)),
                            ))
                            .await
                        {
                            println!("Error when sending the get_item location back 1: {}", e);
                        }
                    }
                    None => {
                        if let Err(e) = tx
                            .send(CommandToServer::GetItemLocationContainer(
                                cmd_id,
                                String::default(),
                                None,
                            ))
                            .await
                        {
                            println!("Error when sending the get_item location back 2 {}", e);
                        }
                    }
                },
                Err(e) => println!(
                    "Error while deserializing json get_item_container_location: {}",
                    e
                ),
            },
            Err(e) => println!("Get item container location error {}", e),
        }
    });
}

pub fn insert_item(
    host: &str,
    tx: &Sender<CommandToServer>,
    id: &str,
    token: &str,
    item: &ContainedItem,
    container_id: &str,
    item_columns_names_types: &[(String, DataType)],
) {
    let container_id = container_id.to_owned();
    let item = item.clone();
    let host = host.to_owned();
    let item_columns_names_types = item_columns_names_types.to_owned();
    let cmd_id = id.to_owned();
    let token = token.to_owned();
    let tx = tx.clone();
    spawn_local(async move {
        let request_data = InsertItemRequest {
            item,
            container_id,
            item_columns_names_types,
        };
        let response = Client::new()
            .post(host + "insert_item")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .send()
            .await;

        match response {
            Ok(_) => {
                if let Err(e) = tx
                    .send(CommandToServer::InsertItem(
                        cmd_id,
                        ContainedItem::default(),
                        String::default(),
                        Vec::new(),
                    ))
                    .await
                {
                    println!("Error when sending the insert_item back: {}", e);
                }
            }
            Err(e) => println!("Error when matching the response insert_item: {}", e),
        }
    });
}

pub fn update_item(
    host: &str,
    tx: &Sender<CommandToServer>,
    cmd_id: &str,
    token: &str,
    item: &ContainedItem,
    item_columns_names_types: &[(String, DataType)],
) {
    let item = item.clone();
    let item_columns_names_types = item_columns_names_types.to_owned();
    let cmd_id = cmd_id.to_owned();
    let tx = tx.clone();
    let host = host.to_owned();
    let token = token.to_owned();
    spawn_local(async move {
        let request_data = InsertItemRequest {
            item,
            container_id: String::new(),
            item_columns_names_types: item_columns_names_types.to_owned(),
        };
        let response = Client::new()
            .post(host + "update_item")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .send()
            .await;

        match response {
            Ok(_) => {
                if let Err(e) = tx
                    .send(CommandToServer::UpdateItem(
                        cmd_id,
                        ContainedItem::default(),
                        Vec::new(),
                    ))
                    .await
                {
                    println!("Error when sending the update_item back: {}", e);
                }
            }
            Err(e) => println!("Error when matching the response update_item: {}", e),
        }
    });
}

pub fn delete_item(
    host: &str,
    tx: &Sender<CommandToServer>,
    cmd_id: &str,
    token: &str,
    item: &ContainedItem,
    container_id: &str,
    delete_from_items: &bool,
) {
    let item = item.clone();
    let cmd_id = cmd_id.to_owned();
    let host = host.to_owned();
    let delete_from_items = *delete_from_items;
    let container_id = container_id.to_owned();
    let token = token.to_owned();
    let tx = tx.clone();
    spawn_local(async move {
        let request_data = DeleteItemRequest {
            item,
            container_id,
            delete_from_items,
        };
        let response = Client::new()
            .post(host + "delete_item")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .send()
            .await;

        match response {
            Ok(_) => {
                if let Err(e) = tx
                    .send(CommandToServer::DeleteItem(
                        cmd_id,
                        ContainedItem::default(),
                        String::new(),
                        false,
                    ))
                    .await
                {
                    println!("Error when sending the delete_item back: {}", e);
                }
            }
            Err(e) => println!("Error when matching the response delete_item: {}", e),
        }
    });
}
