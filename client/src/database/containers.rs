use reqwest::Client;
use serde::Serialize;
use tokio::sync::mpsc::Sender;
use wasm_bindgen_futures::spawn_local;

use crate::{CommandToServer, Container};

#[derive(Serialize)]
struct ContainerRequest {
    container: Container,
}
#[derive(Serialize)]
struct IdRequest {
    id: String,
}

pub fn add_container(
    host: &str,
    tx: &Sender<CommandToServer>,
    id: &str,
    token: &str,
    container: &Container,
) {
    //Takes a container and its master's id and adds it to the database
    //Add the container on the database
    let tx = tx.clone();
    let id = id.to_owned();
    let host = host.to_owned();
    let container = container.clone();
    let token = token.to_owned();
    spawn_local(async move {
        let request_data = ContainerRequest { container };
        let response = Client::new()
            .post(host + "add_container")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .send()
            .await;

        match response {
            Ok(_) => {
                if let Err(e) = tx
                    .send(CommandToServer::AddContainer(id, Container::default()))
                    .await
                {
                    println!("Error when sending add container back: {}", e);
                }
            }
            Err(e) => println!("Add container error {}", e),
        }
    });
}

pub fn delete_container(
    host: &str,
    tx: &Sender<CommandToServer>,
    id: &str,
    token: &str,
    container: &Container,
) {
    //Add the container on the database
    let tx = tx.clone();
    let id = id.to_owned();
    let host = host.to_owned();
    let container = container.clone();
    let token = token.to_owned();
    spawn_local(async move {
        let request_data = ContainerRequest { container };
        let response = Client::new()
            .post(host + "delete_container")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .send()
            .await;

        match response {
            Ok(_) => {
                if let Err(e) = tx
                    .send(CommandToServer::DeleteContainer(id, Container::default()))
                    .await
                {
                    println!("Error when sending delete container back: {}", e);
                }
            }
            Err(e) => println!("Delete container error {}", e),
        }
    });
}

pub fn update_container(
    host: &str,
    tx: &Sender<CommandToServer>,
    id: &str,
    token: &str,
    container: &Container,
) {
    let tx = tx.clone();
    let id = id.to_owned();
    let host = host.to_owned();
    let container = container.clone();
    let token = token.to_owned();
    spawn_local(async move {
        let request_data = ContainerRequest { container };
        let response = Client::new()
            .post(host + "update_container")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .send()
            .await;

        match response {
            Ok(_) => {
                if let Err(e) = tx
                    .send(CommandToServer::UpdateContainer(id, Container::default()))
                    .await
                {
                    println!("Error when sending update container back: {}", e);
                }
            }
            Err(e) => println!("Update container error {}", e),
        }
    });
}

pub fn get_all_slaves(
    host: &str,
    tx: &Sender<CommandToServer>,
    id: &str,
    token: &str,
    source_node_id: &str,
) {
    //Gets the source node slaves and loads each one
    let tx = tx.clone();
    let id = id.to_owned();
    let host = host.to_owned();
    let source_node_id = source_node_id.to_owned();
    let token = token.to_owned();
    spawn_local(async move {
        let request_data = IdRequest { id: source_node_id };
        let response = Client::new()
            .post(host + "get_all_slaves")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .send()
            .await;

        match response {
            Ok(resp) => match resp.json().await {
                Ok(vec) => {
                    if let Err(e) = tx
                        .send(CommandToServer::GetAllSlaves(id, String::default(), vec))
                        .await
                    {
                        println!("Error when sending the slaves back: {}", e);
                    }
                }
                Err(e) => println!("Error while deserializing json get_all_slaves: {}", e),
            },
            Err(e) => println!("Get all slaves error {}", e),
        }
    });
}
