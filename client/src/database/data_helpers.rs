use chrono::Utc;
use egui::ColorImage;
use js_sys::Uint8Array;
use log::{Level, log};
use reqwest::{Client, multipart};
use rfd::{AsyncFileDialog, FileHandle};
use serde::Serialize;
use tokio::sync::mpsc::Sender;
use wasm_bindgen::{JsCast, JsValue, prelude::Closure};
use wasm_bindgen_futures::{JsFuture, spawn_local};
use web_sys::{
    Blob, BlobPropertyBag, Document, FileList, FileReader, HtmlAnchorElement, HtmlInputElement,
    Url, Window,
};

use crate::{CommandToServer, DataType};

#[derive(Serialize)]
struct ColumnRequest {
    column_name: String,
    column_type: DataType,
}

#[derive(Serialize)]
struct UpdateColumnRequest {
    new_column: (String, DataType),
    old_name: String,
}

pub fn get_all_item_ids_not_in_container(
    host: &str,
    tx: &Sender<CommandToServer>,
    id: &str,
    token: &str,
) {
    let id = id.to_owned();
    let host = host.to_owned();
    let tx = tx.clone();
    let token = token.to_owned();
    spawn_local(async move {
        let response = Client::new()
            .post(host + "get_all_item_ids_not_in_container")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;

        match response {
            Ok(resp) => match resp.json::<Vec<String>>().await {
                Ok(vec) => {
                    if let Err(e) = tx
                        .send(CommandToServer::GetAllItemIdsNotInContainer(id, vec))
                        .await
                    {
                        println!(
                            "Error when sending get all item ids not in container back: {}",
                            e
                        );
                    }
                }
                Err(e) => println!(
                    "Error while decoding json in get all item ids not in container {}",
                    e
                ),
            },
            Err(e) => println!("Get all item ids not in container error {}", e),
        }
    });
}

pub fn add_column_to_items(
    host: &str,
    tx: &Sender<CommandToServer>,
    id: &str,
    token: &str,
    column: (String, DataType),
) {
    let id = id.to_owned();
    let host = host.to_owned();
    let tx = tx.clone();
    let token = token.to_owned();

    spawn_local(async move {
        log!(
            Level::Info,
            "Before sending the data: Name: {}, Type: {:?}",
            column.0,
            column.1
        );
        let request_data = ColumnRequest {
            column_name: column.0,
            column_type: column.1,
        };
        let response = Client::new()
            .post(host + "add_column_to_items")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .send()
            .await;

        match response {
            Ok(_) => {
                if let Err(e) = tx
                    .send(CommandToServer::AddField(
                        id,
                        String::default(),
                        DataType::String,
                    ))
                    .await
                {
                    println!("Error when sending the add_column back: {}", e);
                }
            }
            Err(e) => println!("Add column types error {}", e),
        }
    });
}

pub fn delete_column_from_items(
    host: &str,
    tx: &Sender<CommandToServer>,
    id: &str,
    token: &str,
    name: &str,
) {
    let id = id.to_owned();
    let host = host.to_owned();
    let tx = tx.clone();
    let name = name.to_owned();
    let token = token.to_owned();
    spawn_local(async move {
        let request_data = ColumnRequest {
            column_name: name,
            column_type: DataType::String,
        };
        let response = Client::new()
            .post(host + "delete_column_from_items")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .send()
            .await;

        match response {
            Ok(_) => {
                if let Err(e) = tx
                    .send(CommandToServer::DeleteColumnFromItems(
                        id,
                        String::default(),
                    ))
                    .await
                {
                    println!("Error when sending the delete_column back: {}", e);
                }
            }
            Err(e) => println!("Delete column types error {}", e),
        }
    });
}

pub fn update_items_column(
    host: &str,
    tx: &Sender<CommandToServer>,
    id: &str,
    token: &str,
    old_column: &(String, DataType),
    new_name: &str,
) {
    let id = id.to_owned();
    let tx = tx.clone();
    let new_column = old_column.clone();
    let old_name = new_name.to_owned();
    let token = token.to_owned();
    let host = host.to_owned();
    spawn_local(async move {
        let request_data = UpdateColumnRequest {
            new_column,
            old_name,
        };
        let response = Client::new()
            .post(host + "update_items_column")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .send()
            .await;

        match response {
            Ok(_) => {
                if let Err(e) = tx
                    .send(CommandToServer::UpdateItemsColumn(
                        id,
                        (String::default(), DataType::String),
                        String::default(),
                    ))
                    .await
                {
                    println!("Error when sending the rename_column back: {}", e);
                }
            }
            Err(e) => println!("Delete column types error {}", e),
        }
    });
}

#[derive(Serialize, Debug, Clone, PartialEq)]
pub enum ImageSize {
    Small,
    Medium,
    Large,
}

#[derive(Serialize)]
struct ImageRequest {
    image_id: String,
    image_type: String,
    image_size: ImageSize,
}

pub fn get_image_from_server(
    host: &str,
    tx: &Sender<CommandToServer>,
    id: &str,
    token: &str,
    image_id: &str,
    image_type: &str,
    image_size: &ImageSize,
) {
    let id = id.to_owned();
    let tx = tx.clone();
    let token = token.to_owned();
    let host = host.to_owned();
    let image_id = image_id.to_owned();
    let image_type = image_type.to_owned();
    let image_size = image_size.clone();
    spawn_local(async move {
        let request_data = ImageRequest {
            image_id: image_id.to_owned(),
            image_type: image_type.to_owned(),
            image_size: image_size.clone(),
        };
        log::log!(
            Level::Info,
            "Requesting image id: {}, type: {}, size:{:?}",
            image_id,
            image_type,
            image_size
        );
        let response = Client::new()
            .post(host + "images")
            .header("Authorization", format!("Bearer {}", token))
            .json(&request_data)
            .send()
            .await;
        match response {
            Ok(resp) => match resp.bytes().await {
                Ok(bytes) => {
                    // Decode image bytes into a ColorImage
                    if let Ok(img) = image::load_from_memory(&bytes) {
                        let size = [img.width() as usize, img.height() as usize];
                        log::log!(Level::Info, "getting image");
                        let img_rgba = img.to_rgba8();
                        let pixels = img_rgba.as_flat_samples();
                        let image = ColorImage::from_rgba_unmultiplied(size, pixels.as_slice());
                        if let Err(e) = tx
                            .send(CommandToServer::GetImageFromServer(
                                id, image_id, image_type, image_size, image,
                            ))
                            .await
                        {
                            let str = format!("Sending image back err: {}", e).to_string();
                            log::log!(Level::Info, "{}", str);
                        }
                    }
                }
                Err(e) => {
                    println!("Get image from server error {}", e);
                }
            },
            Err(_) => (),
        }
    });
}

pub fn add_image(
    host: &str,
    tx: &Sender<CommandToServer>,
    cmd_id: &str,
    token: &str,
    item_id: &str,
) {
    let item_id = item_id.to_owned();
    let cmd_id = cmd_id.to_owned();
    let host = host.to_owned();
    let token = token.to_owned();
    let tx = tx.clone();
    spawn_local(async move {
        let image_option = AsyncFileDialog::new()
            .add_filter("Supported image formats", &["jpg", "jpeg", "png", "webp"])
            .pick_file()
            .await;
        if let Some(image_file) = image_option {
            let image_data = image_file.read().await;
            let file_name = image_file.file_name();
            let split_name: Vec<&str> = file_name.split(".").collect();
            let mut mime_string: &str = "";
            if split_name.len() == 2 {
                match split_name[1] {
                    "jpeg" | "jpg" => mime_string = "image/jpeg",
                    "png" => mime_string = "image/png",
                    "webp" => mime_string = "image/webp",
                    _ => (),
                }
            }
            if !mime_string.is_empty() {
                let image_field = multipart::Part::bytes(image_data)
                    .file_name(file_name.clone())
                    .mime_str(mime_string);
                if let Ok(image_field) = image_field {
                    let id_field = multipart::Part::text(item_id.clone());
                    let form = multipart::Form::new()
                        .part("item_id", id_field)
                        .part("image", image_field);
                    log::log!(
                        Level::Info,
                        "Adding image id: {}, type:{}",
                        item_id,
                        mime_string
                    );
                    match Client::new()
                        .post(host + "upload_image")
                        .header("Authorization", format!("Bearer {}", token))
                        .multipart(form)
                        .send()
                        .await
                    {
                        Ok(_) => {
                            if let Err(e) = tx
                                .send(CommandToServer::AddImage(
                                    cmd_id.to_string(),
                                    item_id,
                                    split_name[1].to_string(),
                                ))
                                .await
                            {
                                log::log!(Level::Info, "Sending image back err: {}", e);
                                println!("Error when sending the get_image back: {}", e);
                            }
                        }
                        Err(e) => {
                            log!(Level::Error, "Error at the end");
                            println!("Error add image: {}", e)
                        }
                    }
                }
            }
        }
    });
}

pub fn get_backup_from_server(host: &str, tx: &Sender<CommandToServer>, id: &str, token: &str) {
    let id = id.to_owned();
    let tx = tx.clone();
    let token = token.to_owned();
    let host = host.to_owned();
    spawn_local(async move {
        let response = Client::new()
            .post(host + "get_backup")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;
        match response {
            Ok(resp) => match resp.bytes().await {
                Ok(bytes) => {
                    let zip_data = bytes.to_vec();

                    // Create a Blob with the ZIP binary data
                    let blob_parts = js_sys::Uint8Array::from(&zip_data[..]); // Convert Vec<u8> to Uint8Array
                    let blob_property_bag = BlobPropertyBag::new();
                    blob_property_bag.set_type("application/zip");
                    let blob = match Blob::new_with_u8_array_sequence_and_options(
                        &js_sys::Array::of1(&blob_parts).into(),
                        &blob_property_bag,
                    ) {
                        Ok(blob) => blob,
                        Err(e) => {
                            log::log!(Level::Error, "New blob error: {:?}", e);
                            return;
                        }
                    };
                    let timestamp = Utc::now().format("%Y-%m-%d_%H:%M:%S").to_string();
                    let file_name = "Visualoc_Backup_".to_owned() + &timestamp;
                    // Create a temporary URL for the Blob
                    let url = match Url::create_object_url_with_blob(&blob) {
                        Ok(url) => url,
                        Err(e) => {
                            log::log!(Level::Error, "Create object url error: {:?}", e);
                            return;
                        }
                    };

                    // Create an anchor element to trigger the download
                    let window: Window = match web_sys::window() {
                        Some(win) => win,
                        None => {
                            log::log!(Level::Error, "Window is none");
                            return;
                        }
                    };
                    let document: Document = match window.document() {
                        Some(doc) => doc,
                        None => {
                            log::log!(Level::Error, "Documnet is none");
                            return;
                        }
                    };

                    let anchor: HtmlAnchorElement = match document.create_element("a") {
                        Ok(element) => match element.dyn_into() {
                            Ok(anchor) => anchor,
                            Err(e) => {
                                log::log!(Level::Error, "Element dyn into error: {:?}", e);
                                return;
                            }
                        },
                        Err(e) => {
                            log::log!(Level::Error, "Document create element error: {:?}", e);
                            return;
                        }
                    };
                    anchor.set_href(&url);
                    anchor.set_download(&file_name);
                    match document.body() {
                        Some(body) => {
                            if let Err(e) = body.append_child(&anchor) {
                                log::log!(Level::Error, "Append child error: {:?}", e);
                                return;
                            }
                        }
                        None => {
                            log::log!(Level::Error, "No document body");
                            return;
                        }
                    }

                    // Programmatically click the anchor
                    anchor.click();

                    // Clean up
                    match document.body() {
                        Some(body) => {
                            if let Err(e) = body.remove_child(&anchor) {
                                log::log!(Level::Error, "Remove child error: {:?}", e);
                                return;
                            }
                        }
                        None => {
                            log::log!(Level::Error, "No document body");
                            return;
                        }
                    }
                    if let Err(_) = Url::revoke_object_url(&url) {
                        log::log!(Level::Error, "Revoke object url error");
                        return;
                    }
                    if let Err(e) = tx.send(CommandToServer::GetBackup(id)).await {
                        log::log!(Level::Info, "Sending backup vec back err: {}", e);
                    }
                }
                Err(e) => {
                    log::log!(Level::Error, "Backup Bytes error: {}", e);
                }
            },
            Err(e) => {
                log::log!(Level::Error, "Awaiting backup bytes error: {}", e);
            }
        }
    });
}

pub fn pick_dump_file(tx: &Sender<CommandToServer>, cmd_id: &str) {
    let cmd_id = cmd_id.to_owned();
    let tx = tx.clone();
    spawn_local(async move {
        let dump_option = AsyncFileDialog::new()
            .add_filter("Supported file format", &["sql"])
            .pick_file()
            .await;
        if let Some(dump_file) = dump_option {
            if let Err(e) = tx
                .send(CommandToServer::PickBackupDumpFile(
                    cmd_id.to_string(),
                    Some(dump_file),
                ))
                .await
            {
                log::log!(Level::Info, "Sending dump file back err: {}", e);
                println!("Error when sending dump file back: {}", e);
            }
        }
    });
}

pub fn pick_image_folder(tx: &Sender<CommandToServer>, cmd_id: &str) {
    let cmd_id = cmd_id.to_owned();
    let tx = tx.clone();
    let window: Window = match web_sys::window() {
        Some(win) => win,
        None => {
            log::log!(Level::Error, "Window is none");
            return;
        }
    };
    let document: Document = match window.document() {
        Some(doc) => doc,
        None => {
            log::log!(Level::Error, "Documnet is none");
            return;
        }
    };

    let input: HtmlInputElement = match document.get_element_by_id("folder_input") {
        Some(element) => match element.dyn_into::<HtmlInputElement>() {
            Ok(input) => {
                input.set_value("");
                input
            }
            Err(e) => {
                log::log!(Level::Error, "Element dyn into error: {:?}", e);
                return;
            }
        },
        None => {
            log::log!(Level::Error, "Document get element folder_input error");
            return;
        }
    };

    let cloned_input = input.clone();
    // Add event listener to handle file selection
    let closure = Closure::wrap(Box::new(move || {
        if let Some(files) = cloned_input.files() {
            let cmd_id = cmd_id.to_owned();
            let tx = tx.clone();
            spawn_local(async move {
                if let Err(e) = tx
                    .send(CommandToServer::PickBackupImageFolder(
                        cmd_id.to_string(),
                        Some(files),
                    ))
                    .await
                {
                    log::log!(Level::Info, "Sending dump file back err: {}", e);
                    println!("Error when sending dump file back: {}", e);
                }
            });
        }
    }) as Box<dyn FnMut()>);
    input.set_onchange(Some(closure.as_ref().unchecked_ref()));
    closure.forget(); // Leak it to keep it alive

    // Trigger the file picker
    input.click();
}

pub fn upload_backup(
    host: &str,
    tx: &Sender<CommandToServer>,
    cmd_id: &str,
    token: &str,
    dump_file: &FileHandle,
    image_folder: &FileList,
) {
    let cmd_id = cmd_id.to_owned();
    let tx = tx.clone();
    let token = token.to_owned();
    let host = host.to_owned();
    let dump_file = dump_file.clone();
    let image_folder = image_folder.clone();
    spawn_local(async move {
        // Read the dump file
        let dump_file_bytes = dump_file.read().await;
        let file_part = match reqwest::multipart::Part::bytes(dump_file_bytes)
            .file_name(dump_file.file_name())
            .mime_str("application/octet-stream")
        {
            Ok(file_part) => file_part,
            Err(e) => {
                log::log!(Level::Info, "File part from dump err: {}", e);
                return;
            }
        };

        // Create multipart form
        let mut form = reqwest::multipart::Form::new().part("dump_file.sql", file_part);

        // Read and add each file from the folder
        for i in 0..image_folder.length() {
            if let Some(image_file) = image_folder.item(i) {
                let image_bytes = match read_file_to_bytes(&image_file).await {
                    Ok(bytes) => bytes,
                    Err(e) => {
                        log::log!(Level::Info, "Read image to bytes err: {:?}", e);
                        return;
                    }
                };

                let image_part = match reqwest::multipart::Part::bytes(image_bytes)
                    .file_name(image_file.name())
                    .mime_str("image/*")
                {
                    Ok(image_part) => image_part,
                    Err(e) => {
                        log::log!(Level::Info, "Image part from bytes err: {}", e);
                        return;
                    }
                };
                form = form.part(image_file.name(), image_part);
            }
        }

        match Client::new()
            .post(host + "upload_backup")
            .header("Authorization", format!("Bearer {}", token))
            .multipart(form)
            .send()
            .await
        {
            Ok(_) => {
                if let Err(e) = tx
                    .send(CommandToServer::UploadBackup(
                        cmd_id.to_string(),
                        None,
                        None,
                    ))
                    .await
                {
                    log::log!(Level::Info, "Sending image back err: {}", e);
                    println!("Error when sending the get_image back: {}", e);
                }
            }
            Err(e) => {
                log!(Level::Error, "Error at the end");
                println!("Error add image: {}", e)
            }
        }
    });
}

async fn read_file_to_bytes(file: &web_sys::File) -> Result<Vec<u8>, JsValue> {
    let reader = FileReader::new()?;
    let blob = file.slice()?;
    reader.read_as_array_buffer(&blob)?;
    let reader_clone = reader.clone();
    let promise = js_sys::Promise::new(&mut |resolve, reject| {
        let reader = reader.clone();
        let reader_clone = reader_clone.clone();
        let onload = Closure::wrap(Box::new(move || {
            if let Ok(result) = reader_clone.result() {
                resolve.call1(&JsValue::NULL, &result).unwrap();
            } else {
                reject
                    .call1(&JsValue::NULL, &JsValue::from_str("Failed to read file"))
                    .unwrap();
            }
        }) as Box<dyn FnMut()>);
        reader.set_onload(Some(onload.as_ref().unchecked_ref()));
        onload.forget();
    });

    let result = JsFuture::from(promise).await?;
    let array = Uint8Array::new(&result);
    Ok(array.to_vec())
}
