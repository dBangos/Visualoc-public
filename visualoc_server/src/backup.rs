use sqlx::Row;
use std::{io::Write, path::Path};

use axum::{
    Extension,
    extract::{Multipart, State},
    http::StatusCode,
    response::IntoResponse,
};
use sqlx::PgPool;
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
    process::Command,
};
use tokio_util::io::ReaderStream;
use zip::{ZipWriter, write::FileOptions};

use crate::{AppState, add_image_to_database, users::UserContext};

pub async fn serve_backup(
    Extension(user): Extension<UserContext>,
) -> Result<impl IntoResponse, StatusCode> {
    //Creates the backup and sends it to the user
    //Pg_dump and images folder(without the smaller ones) in a zip sent to the user
    let temp_path = "/app/users/".to_owned() + &user.user_id + "/temp";
    //Remove the backup folder in case there are leftover files from a previous operation
    let _ = fs::remove_dir_all(temp_path.clone()).await;
    //Create the backup folder
    if let Err(e) = fs::create_dir_all(temp_path.clone() + "/images").await {
        return Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create backup directory: {}", e),
        )
            .into_response());
    }
    //Dump the users database
    let dump_file = temp_path.clone() + "/dump_file.sql";
    match Command::new("pg_dump")
        .arg("-h")
        .arg("db")
        .arg(format!("--dbname={}", &user.user_id))
        .arg("--file")
        .arg(&dump_file)
        .arg("--format=plain")
        .output()
        .await
    {
        Ok(output) => {
            println!("Output is : {:?}", output);
            println!("output status: {:?}", output.status);
            if !output.status.success() {
                println!("Output failed");
                return Ok((StatusCode::INTERNAL_SERVER_ERROR, "Failed output").into_response());
            }
        }
        Err(e) => {
            println!("Output error: {}", e);
            return Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Output error: {}", e),
            )
                .into_response());
        }
    };

    //Copy over the users images
    let mut files_in_dir =
        match fs::read_dir("/app/users/".to_owned() + &user.user_id + "/images").await {
            Ok(files) => files,
            Err(e) => {
                println!("Reading files error: {}", e);
                return Ok((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Reading files error: {}", e),
                )
                    .into_response());
            }
        };

    while let Ok(Some(file)) = files_in_dir.next_entry().await {
        let path = file.path();
        if path.is_file() {
            if let Some(file_name) = path.file_name() {
                let dest_path = Path::new(&(temp_path.clone() + "/images")).join(file_name);
                if let Err(e) = fs::copy(&path, &dest_path).await {
                    return Ok((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Copying image error: {}", e),
                    )
                        .into_response());
                }
            }
        }
    }

    //Create the zip
    let zip_path = "/app/users/".to_owned() + &user.user_id + "/backup.zip";
    if let Err(e) = zip_dir(&temp_path, &zip_path).await {
        println!("Error creating the zip: {}", e);
        return Ok((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Creating zip error: {}", e),
        )
            .into_response());
    }

    //Load the zip in memory
    let file = match File::open(&zip_path).await {
        Ok(file) => file,
        Err(e) => {
            return Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Loading zip to memory error: {}", e),
            )
                .into_response());
        }
    };

    // Create a stream from the file
    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);

    // Clean up the temporary files
    if let Err(e) = tokio::fs::remove_dir_all(temp_path).await {
        println!("Failed to clean up temp dir: {}", e);
    }
    if let Err(e) = tokio::fs::remove_file(zip_path).await {
        println!("Failed to clean up zip file: {}", e);
    }

    //Send the zip
    let response = axum::http::Response::builder()
        .header(axum::http::header::CONTENT_TYPE, "application/zip")
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(response)
}

async fn zip_dir(dir_path: &str, zip_path: &str) -> std::io::Result<()> {
    // Open the output ZIP file
    let zip_file = std::fs::File::create(zip_path)?;
    let mut zip = ZipWriter::new(zip_file);

    // Recursively add files from the directory
    add_dir_to_zip(dir_path, dir_path, &mut zip).await?;

    // Finalize the ZIP file
    zip.finish()?;
    Ok(())
}

async fn add_dir_to_zip<T: Write + std::io::Seek>(
    base_path: &str,
    dir_path: &str,
    zip: &mut ZipWriter<T>,
) -> std::io::Result<()> {
    let base = Path::new(base_path);
    let path = Path::new(dir_path);

    // Options for files in the ZIP (e.g., compression method)
    let options: FileOptions<()> = FileOptions::default()
        .compression_method(zip::CompressionMethod::Zstd) // Use Deflate compression
        .unix_permissions(0o755); // Set file permissions (Unix-style)
    // Iterate over directory entries
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let entry_path = entry.path();
        let relative_path = match entry_path.strip_prefix(base) {
            Ok(path) => path,
            Err(e) => {
                println!("Error in strip prefix: {}", e);
                return Err(std::io::ErrorKind::InvalidInput.into());
            }
        };
        let name = relative_path.to_str().ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, "Invalid path string")
        })?;

        if entry_path.is_file() {
            // Add a file to the ZIP
            let mut file = match File::open(&entry_path).await {
                Ok(file) => file,
                Err(e) => {
                    println!("Error in open file: {}", e);
                    return Err(e);
                }
            };
            let mut buffer = Vec::new();
            if let Err(e) = file.read_to_end(&mut buffer).await {
                println!("Error in read_to_end file: {}", e);
                return Err(e);
            }
            zip.start_file(name, options)?;
            zip.write_all(&buffer)?;
        } else if entry_path.is_dir() {
            // Recursively add subdirectories
            if let Some(entry_path) = entry_path.to_str() {
                if let Err(e) = Box::pin(add_dir_to_zip(base_path, entry_path, zip)).await {
                    println!("Error in recursive call: {}", e);
                    return Err(e);
                }
            } else {
                println!("Error in recursive call path is none");
                return Err(std::io::ErrorKind::InvalidInput.into());
            }
        }
    }
    Ok(())
}

pub async fn restore_to_user_backup(
    State(state): State<AppState>,
    Extension(user): Extension<UserContext>,
    mut multipart: Multipart,
) -> Result<(), (StatusCode, String)> {
    println!("In backup");
    let temp_path = "/app/users/".to_owned() + &user.user_id + "/temp";
    //Remove the backup folder in case there are leftover files from a previous operation
    let _ = fs::remove_dir_all(temp_path.clone()).await;
    //Create the backup folder
    if let Err(e) = fs::create_dir_all(temp_path.clone() + "/images").await {
        println!("Failed to create dir images: {:?}", e);
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create backup directory: {}", e),
        ));
    }
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        println!("Upload image error {}", e);
        (StatusCode::BAD_REQUEST, e.to_string())
    })? {
        if let Some(file_name) = field.name() {
            let filename: Vec<&str> = file_name.split(".").collect();
            let file_path = if filename.len() == 2 {
                let file_extension = filename[1].to_owned();
                if file_extension == "webp"
                    || file_extension == "jpg"
                    || file_extension == "png"
                    || file_extension == "jpeg"
                    || file_extension == "gif"
                {
                    temp_path.clone() + "/images/" + file_name
                } else if file_extension == "sql" {
                    temp_path.clone() + "/" + file_name
                } else {
                    println!("Error 1, File extension: {}", file_extension);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to read file name".to_string(),
                    ));
                }
            } else {
                println!("Error 2");
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Failed to read file name".to_string(),
                ));
            };
            let data = match field.bytes().await {
                Ok(data) => data,
                Err(e) => {
                    println!("Error 3");
                    return Err((StatusCode::BAD_REQUEST, e.to_string()));
                }
            };

            // Handle potential folder structure from the filename
            if let Some(parent_dir) = std::path::Path::new(&file_path).parent() {
                if let Err(e) = fs::create_dir_all(parent_dir).await {
                    println!("Error 4");
                    return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
                }
            }

            // Write the file
            let mut file = match fs::File::create(&file_path).await {
                Ok(file) => file,
                Err(e) => {
                    println!("Error 5");
                    return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
                }
            };
            if let Err(e) = file.write(&data).await {
                println!("Error 6");
                return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
            }
        }
    }
    if let Err(e) = restore_database(
        &user.user_id,
        &(temp_path.clone() + "/dump_file.sql"),
        &state.master_pool,
        &user.db_pool,
    )
    .await
    {
        return Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to restore from backup: {}", e),
        ));
    }
    tokio::spawn(async move {
        let _ = restore_images(&user).await;
    });
    Ok(())
}

async fn restore_database(
    db_name: &str,
    dump_file_path: &str,
    master_pool: &PgPool,
    user_pool: &PgPool,
) -> Result<(), String> {
    println!("Gets in restore");
    //Get the user's username
    let username_row = match sqlx::query("SELECT username FROM users where id = $1")
        .bind(&db_name)
        .fetch_one(master_pool)
        .await
    {
        Ok(username_row) => username_row,
        Err(e) => {
            return Err(format!(
                "Couldn't find username when restoring database: {}",
                e
            ));
        }
    };
    let username: &str = username_row.get("username");
    //Empty the database by dropping and recreating the public schema
    if let Err(e) = sqlx::query("DROP SCHEMA public CASCADE;")
        .execute(user_pool)
        .await
    {
        println!("Drop schema error: {}", e);
        return Err(format!("Drop schema error: {}", e));
    }
    if let Err(e) = sqlx::query("CREATE SCHEMA public;")
        .execute(user_pool)
        .await
    {
        println!("Create schema error: {}", e);
        return Err(format!("Create schema error: {}", e));
    }

    if let Err(e) = sqlx::query(&format!(r#"GRANT ALL ON SCHEMA public TO "{}";"#, username))
        .execute(user_pool)
        .await
    {
        println!("Grant schema error: {}", e);
        return Err(format!("Grant schema error: {}", e));
    }
    // Run psql to restore the database
    let output = Command::new("psql")
        .arg("-h")
        .arg("db")
        .arg("-d")
        .arg(db_name)
        .arg("-f")
        .arg(dump_file_path)
        .output()
        .await;
    return match output {
        Ok(output) => {
            println!("{:?}", output);
            if output.status.success() {
                println!("Database restored successfully");
                Ok(())
            } else {
                return Err(format!(
                    "Restore database output error: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
        }
        Err(e) => {
            println!("error in restore database: {}", e);
            return Err(format!("Restore database error: {}", e));
        }
    };
}

async fn restore_images(user: &UserContext) -> Result<(), String> {
    let temp_path = "/app/users/".to_owned() + &user.user_id + "/temp";
    let images_path = "/app/users/".to_owned() + &user.user_id + "/images";
    let image_extensions = vec!["png", "jpg", "jpeg", "gif", "webp"];
    //Remove the backup folder in case there are leftover files from a previous operation
    let _ = fs::remove_dir_all(images_path.clone()).await;
    let _ = fs::create_dir_all(images_path.clone() + "/small").await;
    let _ = fs::create_dir(images_path.clone() + "/medium").await;
    let mut images = match fs::read_dir(temp_path.clone() + "/images").await {
        Ok(images) => images,
        Err(e) => {
            println!("Error when reading images: {}", e);
            return Err(format!("Error when reading images: {}", e));
        }
    };
    while let Ok(Some(entry)) = images.next_entry().await {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                let ext = ext.to_str().unwrap_or("").to_lowercase();
                if image_extensions.contains(&ext.as_str()) {
                    if let Some(name) = path.clone().file_name() {
                        if let Some(name) = name.to_str() {
                            let split_string: Vec<&str> = name.split(".").collect();
                            let item_id = split_string[0].to_string();
                            let data = match fs::read(path).await {
                                Ok(data) => data,
                                Err(e) => {
                                    println!("Error when reading image: {}", e);
                                    return Err(format!("Error when reading image: {}", e));
                                }
                            };
                            let _ =
                                add_image_to_database(&data.into(), &user, &item_id, &ext).await;
                        }
                    }
                }
            }
        }
    }
    //Remove the temp folder
    let _ = fs::remove_dir_all(temp_path).await;
    return Ok(());
}
