use std::{collections::BTreeSet, sync::Arc, time::Duration};

use axum::{
    Extension, Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Multipart},
    http::{HeaderValue, Method, StatusCode},
    middleware,
    response::IntoResponse,
    routing::post,
};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use sqlx::{Column, PgPool, Row, postgres::PgRow};
use tokio::{
    fs::{self, File},
    time::sleep,
};
use tokio_util::io::ReaderStream;
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    limit::RequestBodyLimitLayer,
};
use tracing::{Level, info};
use users::{UserContext, auth_middleware, cleanup_inactive_pools};
use uuid::Uuid;

mod backup;
mod users;

#[derive(Serialize, Deserialize)]
enum DataType {
    String,
    Integer,
    Float,
    Bool,
    Percentage,
    Text,
    List(Vec<String>),
    Gallery,
}

#[derive(Serialize, Deserialize)]
struct ContainedItem {
    id: String,
    name: String,
    image_type: String,
    //Dynamic fields contain the user defined variables
    //Each variable gets defined by the extra database columns
    string_vars: Vec<String>,
    int_vars: Vec<i32>,
    float_vars: Vec<f32>,
}
impl Default for ContainedItem {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "".to_string(),
            image_type: String::new(),
            string_vars: Vec::new(),
            int_vars: Vec::new(),
            float_vars: Vec::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Container {
    id: String,
    master: String,
    slaves: BTreeSet<String>,
    name: String,
    corners: [f32; 4],
    image_type: String,
    contained_items: BTreeSet<String>,
}

#[derive(Deserialize)]
struct IdVectorRequest {
    id_vec: Vec<String>,
}

#[derive(Deserialize)]
struct SearchItemsRequest {
    column_name: String,
    search_string: String,
}

#[derive(Deserialize)]
struct IdRequest {
    id: String,
}

#[derive(Deserialize)]
struct InsertItemRequest {
    container_id: String,
    item: ContainedItem,
    item_columns_names_types: Vec<(String, DataType)>,
}

#[derive(Deserialize)]
struct DeleteItemRequest {
    container_id: String,
    item: ContainedItem,
    delete_from_items: bool,
}

#[derive(Deserialize)]
struct ContainerRequest {
    container: Container,
}

#[derive(Deserialize)]
struct ColumnRequest {
    column_name: String,
    column_type: DataType,
}

#[derive(Deserialize)]
struct UpdateColumnRequest {
    new_column: (String, DataType),
    old_name: String,
}

pub fn corners_to_string(corners: &[f32; 4]) -> String {
    let mut result = String::new();
    for item in corners {
        result += &item.to_string();
        result.push('@');
    }
    return result;
}

pub fn string_to_corners(input: &str) -> [f32; 4] {
    let mut result = [0.0; 4];
    for (index, string) in input.split('@').enumerate() {
        if !string.is_empty() {
            if let Ok(num) = string.parse() {
                result[index] = num;
            }
        }
    }
    return result;
}

pub fn set_to_string(set: &BTreeSet<String>) -> String {
    let mut result = String::new();
    for item in set {
        result += item;
        result.push('@');
    }
    return result;
}

pub fn string_to_set(input: &str) -> BTreeSet<String> {
    let mut result = BTreeSet::new();
    for string in input.split('@') {
        if !string.is_empty() {
            result.insert(string.to_owned());
        }
    }
    return result;
}

#[derive(Clone)]
struct AppState {
    master_pool: PgPool,
    secret: String,
    master_db_string: String,
    user_pools: Arc<DashMap<String, Arc<PgPool>>>,
    token_belonging_to_user: Arc<DashMap<String, String>>,
}

// const DOMAIN: &str = "https://visualoc.com";
const DOMAIN: &str = "http://127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    // initialize tracing for logging
    tracing_subscriber::fmt().with_max_level(Level::INFO).init();
    let pg_password = std::env::var("PGPASSWORD").unwrap_or("".into());
    let pg_user = std::env::var("PGUSER").unwrap_or("".into());
    let secret = std::env::var("SECRET").unwrap_or("".into());
    // let master_db_string = "postgres://postgres@localhost:5432".to_string();
    let master_db_string = format!("postgres://{}:{}@db:5432", pg_user, pg_password);
    let pool = setup_master_database(&(master_db_string.clone() + "/master")).await;
    let state = AppState {
        master_pool: pool,
        secret,
        master_db_string,
        user_pools: Arc::new(DashMap::new()),
        token_belonging_to_user: Arc::new(DashMap::new()),
    };
    tokio::spawn(cleanup_inactive_pools(state.clone()));
    println!("Connected to the database!");
    let public_routes = Router::new()
        .route("/register", post(users::register))
        .route("/login", post(users::login))
        .route("/authenticate", post(users::logged_in_authentication))
        .layer(
            CorsLayer::new()
                .allow_origin(DOMAIN.parse::<HeaderValue>().unwrap())
                .allow_methods(Method::POST)
                .allow_headers(Any),
        )
        .with_state(state.clone());

    let private_routes = Router::new()
        .route("/get_multiple_items", post(get_multiple_items))
        .route("/search_items", post(search_items))
        .route(
            "/get_item_container_location",
            post(get_item_container_location),
        )
        .route("/insert_item", post(insert_item))
        .route("/update_item", post(update_item))
        .route("/delete_item", post(delete_item))
        .route("/add_container", post(add_container))
        .route("/delete_container", post(delete_container))
        .route("/update_container", post(update_container))
        .route(
            "/get_all_item_ids_not_in_container",
            post(get_all_item_ids_not_in_container),
        )
        .route("/add_column_to_items", post(add_column_to_items))
        .route("/delete_column_from_items", post(delete_column_from_items))
        .route("/update_items_column", post(update_items_column))
        .route("/get_item_column_types", post(get_dynamic_fields))
        .route("/get_all_slaves", post(get_all_slaves))
        .route("/upload_image", post(upload_image))
        .route("/images", post(serve_image))
        .route("/get_backup", post(backup::serve_backup))
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        )))
        .layer(
            CorsLayer::new()
                .allow_origin(DOMAIN.parse::<HeaderValue>().unwrap())
                .allow_methods(Method::POST)
                .allow_headers(Any),
        )
        .with_state(state.clone());

    let different_rate_routes = Router::new()
        .route("/upload_backup", post(backup::restore_to_user_backup))
        .layer(ServiceBuilder::new().layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        )))
        .layer(
            CorsLayer::new()
                .allow_origin(DOMAIN.parse::<HeaderValue>().unwrap())
                .allow_methods(Method::POST)
                .allow_headers(Any),
        )
        .layer(DefaultBodyLimit::disable())
        .layer(ServiceBuilder::new().layer(RequestBodyLimitLayer::new(200 * 1024 * 1024)))
        .with_state(state.clone());

    let app = Router::new()
        .merge(private_routes)
        .merge(public_routes)
        .merge(different_rate_routes)
        .with_state(state);
    // run our app with hyper, listening globally on port 8010
    let listener = tokio::net::TcpListener::bind("0.0.0.0:8010").await.unwrap();
    info!("Server is running on http://0.0.0.0:8010");
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

async fn setup_master_database(connection_string: &str) -> PgPool {
    // println!("connection string: {}", connection_string);
    let pool = match connect_with_retry(connection_string).await {
        Ok(pool) => pool,
        Err(e) => {
            println!("Error when connecting: {:?}", e);
            connect_with_retry(connection_string).await.unwrap()
        }
    };
    sqlx::query(
        r#"
        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            username TEXT NOT NULL UNIQUE,
            email TEXT NOT NULL UNIQUE,
            password_hash TEXT NOT NULL,
            connection_string TEXT NOT NULL
            )
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    return pool;
}

async fn connect_with_retry(url: &str) -> Result<PgPool, sqlx::Error> {
    for _ in 0..10 {
        match PgPool::connect(url).await {
            Ok(pool) => return Ok(pool),
            Err(_) => sleep(Duration::from_secs(1)).await,
        }
    }
    PgPool::connect(url).await // Final attempt
}
async fn get_multiple_items(
    Extension(user): Extension<UserContext>,
    Json(payload): Json<IdVectorRequest>,
) -> Result<Json<Vec<ContainedItem>>, StatusCode> {
    println!("in get multiple items");
    let rows = sqlx::query("SELECT * FROM items WHERE id = ANY($1)")
        .bind(payload.id_vec)
        .fetch_all(&*user.db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let items = rows
        .into_iter()
        .map(|row: PgRow| {
            let mut item = ContainedItem::default();
            row.columns().iter().for_each(|column| {
                match (column.name(), &column.type_info().to_string() as &str) {
                    ("id", "TEXT") => item.id = row.get("id"),
                    ("name", "TEXT") => item.name = row.get("name"),
                    ("image_type", "TEXT") => item.image_type = row.get("image_type"),
                    (var, "TEXT") => item
                        .string_vars
                        .push(row.try_get(var).unwrap_or(String::default())),
                    (var, "INT4") => item.int_vars.push(row.try_get(var).unwrap_or(0)),
                    (var, "FLOAT4") => item.float_vars.push(row.try_get(var).unwrap_or(0.0)),
                    _ => (),
                }
            });
            return item;
        })
        .collect();
    Ok(Json(items))
}

async fn search_items(
    Extension(user): Extension<UserContext>,
    Json(mut payload): Json<SearchItemsRequest>,
) -> Result<Json<Vec<ContainedItem>>, StatusCode> {
    if payload.column_name.chars().any(|x| !x.is_alphanumeric()) {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    if payload.column_name == "Name" {
        payload.column_name = "name".to_string();
    }

    let query = format!(
        r#"SELECT * FROM items WHERE "{}"::text ILIKE '%'|| $2 || '%'"#,
        payload.column_name
    );
    match sqlx::query(&query)
        .bind(payload.column_name)
        .bind(payload.search_string)
        .fetch_all(&*user.db_pool)
        .await
    {
        Ok(rows) => {
            let items = rows
                .into_iter()
                .map(|row: PgRow| {
                    let mut item = ContainedItem::default();
                    row.columns().iter().for_each(|column| {
                        match (column.name(), &column.type_info().to_string() as &str) {
                            ("id", "TEXT") => item.id = row.get("id"),
                            ("name", "TEXT") => item.name = row.get("name"),
                            ("image_type", "TEXT") => item.image_type = row.get("image_type"),
                            (var, "TEXT") => item
                                .string_vars
                                .push(row.try_get(var).unwrap_or(String::default())),
                            (var, "INT4") => item.int_vars.push(row.try_get(var).unwrap_or(0)),
                            (var, "FLOAT4") => {
                                item.float_vars.push(row.try_get(var).unwrap_or(0.0))
                            }

                            _ => (),
                        }
                    });
                    return item;
                })
                .collect();
            Ok(Json(items))
        }
        Err(e) => {
            println!("Searching items error {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

async fn get_item_container_location(
    Extension(user): Extension<UserContext>,
    Json(payload): Json<IdRequest>,
) -> Result<Json<Option<(Container, Container)>>, StatusCode> {
    println!("in get item container");
    let row = sqlx::query("SELECT * FROM containers WHERE contained_items LIKE '%' || $1 || '%'")
        .bind(payload.id)
        .fetch_optional(&*user.db_pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match row {
        Some(container_row) => {
            let container = pgrow_to_container(container_row);
            let row = sqlx::query("SELECT * FROM containers WHERE slaves LIKE '%' || $1 || '%'")
                .bind(&container.id)
                .fetch_optional(&*user.db_pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            match row {
                Some(location_row) => {
                    return Ok(Json(Some((container, pgrow_to_container(location_row)))));
                }
                None => return Ok(Json(None)),
            }
        }
        None => return Ok(Json(None)),
    }
}

fn pgrow_to_container(container_row: PgRow) -> Container {
    let slaves: String = container_row.get("slaves");
    let corners: String = container_row.get("corners");
    let contained_items: String = container_row.get("contained_items");
    let container = Container {
        id: container_row.get("id"),
        master: container_row.get("master"),
        image_type: container_row.get("image_type"),
        name: container_row.get("name"),
        slaves: string_to_set(&slaves),
        corners: string_to_corners(&corners),
        contained_items: string_to_set(&contained_items),
    };
    return container;
}

#[derive(Clone)]
enum DynamicFieldValue {
    Integer(i32),
    Float(f32),
    Text(String),
}

async fn insert_item(
    Extension(user): Extension<UserContext>,
    Json(payload): Json<InsertItemRequest>,
) -> Result<StatusCode, StatusCode> {
    println!("in insert item");
    let mut value_vec: Vec<DynamicFieldValue> = Vec::new();
    let mut column_names: Vec<String> = Vec::new();
    value_vec.push(DynamicFieldValue::Text(payload.item.id.clone()));
    column_names.push("id".into());
    value_vec.push(DynamicFieldValue::Text(payload.item.name));
    column_names.push("name".into());
    value_vec.push(DynamicFieldValue::Text(payload.item.image_type));
    column_names.push("image_type".into());
    let mut string_index: usize = 0;
    let mut i32_index: usize = 0;
    let mut f32_index: usize = 0;
    //Adding each dynamic fields name and value
    for (column_name, column_type) in payload.item_columns_names_types {
        column_names.push(column_name.clone());
        match column_type {
            DataType::String | DataType::List(_) | DataType::Text | DataType::Gallery => {
                value_vec.push(DynamicFieldValue::Text(
                    payload.item.string_vars[string_index].clone(),
                ));
                string_index += 1;
            }
            DataType::Float | DataType::Percentage => {
                value_vec.push(DynamicFieldValue::Float(payload.item.float_vars[f32_index]));
                f32_index += 1;
            }
            DataType::Integer | DataType::Bool => {
                value_vec.push(DynamicFieldValue::Integer(payload.item.int_vars[i32_index]));
                i32_index += 1;
            }
        }
    }
    // Create the query
    let mut value_clause = Vec::new();
    for index in 1..=column_names.len() {
        value_clause.push(format!("${}", index));
    }
    let column_clause = column_names.join(r#"", ""#);
    let value_clause = value_clause.join(", ");
    let items_query = format!(
        r#"INSERT INTO items ("{}") VALUES ({})"#,
        column_clause, value_clause
    );

    //Add item_id to containers containeditems
    let container_query = sqlx::query(
        r#"
    UPDATE containers
    SET contained_items = contained_items || $1 || '@'
    WHERE id = $2
    "#,
    )
    .bind(payload.item.id)
    .bind(payload.container_id)
    .execute(&*user.db_pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .rows_affected();

    if container_query != 1 {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    //Add the row to the items table
    let mut query = sqlx::query(&items_query);
    //Bind to values
    for val in value_vec.clone() {
        match val {
            DynamicFieldValue::Text(val) => query = query.bind(val),
            DynamicFieldValue::Integer(val) => query = query.bind(val),
            DynamicFieldValue::Float(val) => query = query.bind(val),
        };
    }
    match query.execute(&*user.db_pool).await {
        Ok(_) => Ok(StatusCode::CREATED),

        Err(e) => {
            println!("Insert item error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn update_item(
    Extension(user): Extension<UserContext>,
    Json(payload): Json<InsertItemRequest>,
) -> Result<StatusCode, StatusCode> {
    println!("in update item");
    let mut value_vec: Vec<DynamicFieldValue> = Vec::new();
    let mut column_names: Vec<String> = Vec::new();
    value_vec.push(DynamicFieldValue::Text(payload.item.name));
    column_names.push("name".into());
    value_vec.push(DynamicFieldValue::Text(payload.item.image_type));
    column_names.push("image_type".into());
    let mut string_index: usize = 0;
    let mut i32_index: usize = 0;
    let mut f32_index: usize = 0;
    //Adding each dynamic fields name and value
    for (column_name, column_type) in payload.item_columns_names_types {
        column_names.push(column_name.clone());
        match column_type {
            DataType::String | DataType::List(_) | DataType::Text | DataType::Gallery => {
                value_vec.push(DynamicFieldValue::Text(
                    payload.item.string_vars[string_index].clone(),
                ));
                string_index += 1;
            }
            DataType::Float | DataType::Percentage => {
                value_vec.push(DynamicFieldValue::Float(payload.item.float_vars[f32_index]));
                f32_index += 1;
            }
            DataType::Integer | DataType::Bool => {
                value_vec.push(DynamicFieldValue::Integer(payload.item.int_vars[i32_index]));
                i32_index += 1;
            }
        }
    }
    // Create the query
    let mut set_clause = Vec::new();
    let mut param_idx = 1;
    for name in &column_names {
        set_clause.push(format!(r#""{}" = ${}"#, name, param_idx));
        param_idx += 1;
    }
    let set_clause = set_clause.join(", ");
    let query = format!(
        r#"UPDATE items SET {} WHERE id = ${}"#,
        set_clause, param_idx
    );
    //Add the row to the items table
    let mut query = sqlx::query(&query);
    //Bind to values
    for val in value_vec.clone() {
        match val {
            DynamicFieldValue::Text(val) => query = query.bind(val),
            DynamicFieldValue::Integer(val) => query = query.bind(val),
            DynamicFieldValue::Float(val) => query = query.bind(val),
        };
    }
    match query.bind(payload.item.id).execute(&*user.db_pool).await {
        Ok(_) => Ok(StatusCode::OK),
        Err(e) => {
            println!("Update item error: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

async fn delete_item(
    Extension(user): Extension<UserContext>,
    Json(payload): Json<DeleteItemRequest>,
) -> Result<StatusCode, StatusCode> {
    println!("in delete item");
    //Remove the items image
    remove_image(&user.user_id, &payload.item.id).await;
    let container_query = sqlx::query(
        r#"
        UPDATE containers
        SET
        contained_items = REPLACE(contained_items, $1, '')
        WHERE id = $2
        "#,
    )
    .bind(payload.item.id.clone() + "@")
    .bind(&payload.container_id)
    .execute(&*user.db_pool)
    .await;

    if container_query.is_err() {
        println!("Delete item error 1: {}", container_query.unwrap_err());
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    if payload.delete_from_items {
        match sqlx::query(
            r#"
            DELETE FROM items WHERE id = $1
            "#,
        )
        .bind(payload.item.id)
        .execute(&*user.db_pool)
        .await
        {
            Ok(_) => {
                return Ok(StatusCode::OK);
            }
            Err(e) => {
                println!("Update item error: {}", e);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    }

    Ok(StatusCode::OK)
}

async fn add_container(
    Extension(user): Extension<UserContext>,
    Json(payload): Json<ContainerRequest>,
) -> Result<StatusCode, StatusCode> {
    println!("in add container");
    let serialized_slaves = set_to_string(&payload.container.slaves);
    let serialized_corners = corners_to_string(&payload.container.corners);
    let serialized_items = set_to_string(&payload.container.contained_items);
    match sqlx::query(
        r#"
        UPDATE containers
        SET slaves = slaves || $1 || '@'
        WHERE id = $2
        "#,
    )
    .bind(&payload.container.id)
    .bind(&payload.container.master)
    .execute(&*user.db_pool)
    .await
    {
        Ok(_) => {
            match sqlx::query(
                r#"
                INSERT INTO containers (id,name,master,slaves,corners,image_type,contained_items) VALUES ($1, $2, $3, $4, $5, $6, $7) ON CONFLICT DO NOTHING"#,
            ).bind(&payload.container.id)
            .bind(payload.container.name)
            .bind(payload.container.master)
            .bind(serialized_slaves)
            .bind(serialized_corners)
            .bind(payload.container.image_type)
            .bind(serialized_items)
            .execute(&*user.db_pool)
            .await
            {
                Ok(_)=>{
                return Ok(StatusCode::OK);}
                Err(e) =>{

                    println!("Add container error {}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
        Err(e) =>{
            println!("Adding container to master's slaves error {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

async fn delete_container(
    Extension(user): Extension<UserContext>,
    Json(payload): Json<ContainerRequest>,
) -> Result<StatusCode, StatusCode> {
    println!("in delete container");
    //Remove the container from its master
    match sqlx::query(
        r#"
        UPDATE containers
        SET
        slaves = REPLACE(slaves, $1, '')
        WHERE id = $2
        "#,
    )
    .bind(payload.container.id.clone() + "@")
    .bind(&payload.container.master)
    .execute(&*user.db_pool)
    .await
    {
        Ok(_) => {
            //Remove the containers image
            remove_image(&user.user_id, &payload.container.id).await;
            //Remove the containers slaves
            for slave_id in payload.container.slaves {
                match sqlx::query(
                    r#"
                    DELETE FROM containers WHERE id = $1
                    "#,
                )
                .bind(slave_id)
                .execute(&*user.db_pool)
                .await
                {
                    Ok(_) => (),
                    Err(e) => println!("Delete container's slaves error {}", e),
                }
            }
            //Remove the container row
            match sqlx::query(
                r#"
                DELETE FROM containers WHERE id = $1
                "#,
            )
            .bind(&payload.container.id)
            .execute(&*user.db_pool)
            .await
            {
                Ok(_) => {
                    return Ok(StatusCode::OK);
                }
                Err(e) => {
                    println!("Delete container error {}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
        Err(e) => {
            println!("Deleting container from master's slaves error {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

async fn update_container(
    Extension(user): Extension<UserContext>,
    Json(payload): Json<ContainerRequest>,
) -> Result<StatusCode, StatusCode> {
    println!("in update container");
    let serialized_slaves = set_to_string(&payload.container.slaves);
    let serialized_corners = corners_to_string(&payload.container.corners);
    let serialized_items = set_to_string(&payload.container.contained_items);
    match sqlx::query(
        r#"
        UPDATE containers SET name=$1, master=$2, slaves=$3, corners=$4, image_type=$5, contained_items=$6 WHERE id=$7
        "#,
    )
    .bind(&payload.container.name)
    .bind(&payload.container.master)
    .bind(&serialized_slaves)
    .bind(&serialized_corners)
    .bind(&payload.container.image_type)
    .bind(&serialized_items)
    .bind(&payload.container.id)
    .execute(&*user.db_pool)
    .await
    {
        Ok(_) => {
            return Ok(StatusCode::OK);
        }
        Err(e) => {
            println!("Upadte container error {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

async fn get_all_item_ids_not_in_container(
    Extension(user): Extension<UserContext>,
) -> Result<Json<Vec<String>>, StatusCode> {
    println!("in get all item ids not in container");
    //Get all item_ids in containers
    let query = "SELECT contained_items FROM containers";
    match sqlx::query(query).fetch_all(&*user.db_pool).await {
        Ok(vec) => {
            //Get all item_ids
            let query = "SELECT id FROM items";
            match sqlx::query(query).fetch_all(&*user.db_pool).await {
                Ok(vec2) => {
                    let in_container_ids: Vec<String> =
                        vec.into_iter().map(|x| x.get("contained_items")).collect();
                    let in_container_ids: Vec<String> = in_container_ids
                        .join("")
                        .split("@")
                        .filter(|x| !x.is_empty())
                        .map(|x| x.to_string())
                        .collect();
                    let all_item_ids: Vec<String> = vec2.into_iter().map(|x| x.get("id")).collect();
                    let result: Vec<String> = all_item_ids
                        .into_iter()
                        .filter(|x| !in_container_ids.contains(x))
                        .collect();
                    return Ok(Json(result));
                }
                Err(e) => {
                    println!("Get all item ids not in container error 1 : {}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
        Err(e) => {
            println!("Get all item ids not in container error 2 : {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

async fn add_column_to_items(
    Extension(user): Extension<UserContext>,
    Json(payload): Json<ColumnRequest>,
) -> Result<StatusCode, StatusCode> {
    println!("in add column to items");
    if payload.column_name.chars().any(|x| !x.is_alphanumeric()) {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let list_val: String; //Need a let binding for the joined list to not be dropped early
    let (item_query, column_type_string) = match payload.column_type {
        DataType::String => (
            format!(
                r#"ALTER TABLE items ADD COLUMN "{}" TEXT"#,
                payload.column_name,
            ),
            "text",
        ),
        DataType::List(val) => {
            list_val = "list,".to_owned() + &val.join(",");
            (
                format!(
                    r#"ALTER TABLE items ADD COLUMN "{}" TEXT"#,
                    payload.column_name,
                ),
                list_val.as_str(),
            )
        }
        DataType::Gallery => (
            format!(
                r#"ALTER TABLE items ADD COLUMN "{}" TEXT"#,
                payload.column_name,
            ),
            "gallery",
        ),
        DataType::Text => (
            format!(
                r#"ALTER TABLE items ADD COLUMN "{}" TEXT"#,
                payload.column_name,
            ),
            "paragraph",
        ),
        DataType::Integer => (
            format!(
                r#"ALTER TABLE items ADD COLUMN "{}" INT"#,
                payload.column_name
            ),
            "integer",
        ),
        DataType::Bool => (
            format!(
                r#"ALTER TABLE items ADD COLUMN "{}" INT"#,
                payload.column_name
            ),
            "bool",
        ),
        DataType::Float => (
            format!(
                r#"ALTER TABLE items ADD COLUMN "{}" REAL"#,
                payload.column_name
            ),
            "float",
        ),
        DataType::Percentage => (
            format!(
                r#"ALTER TABLE items ADD COLUMN "{}" REAL"#,
                payload.column_name
            ),
            "percentage",
        ),
    };
    if let Err(e) = sqlx::query(&item_query).execute(&*user.db_pool).await {
        println!("Add column to items error {}", e);
    }

    //Add the dynamic_field to the dynamic_fields table
    match sqlx::query(
        r#"
        INSERT INTO dynamic_fields (name, type) VALUES ($1, $2) ON CONFLICT DO NOTHING
        "#,
    )
    .bind(payload.column_name)
    .bind(column_type_string)
    .execute(&*user.db_pool)
    .await
    {
        Ok(_) => {
            return Ok(StatusCode::OK);
        }
        Err(e) => {
            println!("Add column to dynamic_fields error {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

async fn delete_column_from_items(
    Extension(user): Extension<UserContext>,
    Json(payload): Json<ColumnRequest>,
) -> Result<StatusCode, StatusCode> {
    println!("in delete column from items");
    println!("column_name:{}", payload.column_name);
    if payload.column_name.chars().any(|x| !x.is_alphanumeric()) {
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let query = format!(r#"ALTER TABLE items DROP COLUMN "{}""#, payload.column_name);
    if let Err(e) = sqlx::query(&query).execute(&*user.db_pool).await {
        println!("Delete column from items error {}", e);
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    //Delete the field from dynamic_fields table
    match sqlx::query(
        r#"
        DELETE FROM dynamic_fields WHERE name = $1
        "#,
    )
    .bind(payload.column_name)
    .execute(&*user.db_pool)
    .await
    {
        Ok(_) => {
            return Ok(StatusCode::OK);
        }
        Err(e) => {
            println!("Delete column from dynamic_fields error {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

async fn update_items_column(
    Extension(user): Extension<UserContext>,
    Json(payload): Json<UpdateColumnRequest>,
) -> Result<StatusCode, StatusCode> {
    let query = format!(
        r#"ALTER TABLE items RENAME COLUMN "{}" TO "{}""#,
        payload.old_name, payload.new_column.0
    );
    if let Err(e) = sqlx::query(&query).execute(&*user.db_pool).await {
        println!("Update column from items error {}", e);
    }
    //If the column is a list, update the list too
    match payload.new_column.1 {
        DataType::List(val) => {
            let list_val = "list,".to_owned() + &val.join(",");
            println!("list_val: {}", list_val);
            if let Err(e) = sqlx::query(
                r#"
                UPDATE dynamic_fields SET type=$1 WHERE name=$2
                "#,
            )
            .bind(list_val)
            .bind(&payload.old_name)
            .execute(&*user.db_pool)
            .await
            {
                println!("Update field list in dynamic_fields error {}", e);
            }
        }
        _ => (),
    }
    //Change the name in the dynamic_fields table
    match sqlx::query(
        r#"
        UPDATE dynamic_fields SET name=$1 WHERE name=$2
        "#,
    )
    .bind(payload.new_column.0)
    .bind(payload.old_name)
    .execute(&*user.db_pool)
    .await
    {
        Ok(_) => {
            return Ok(StatusCode::OK);
        }
        Err(e) => {
            println!("Update field in dynamic_fields error {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

async fn get_dynamic_fields(
    Extension(user): Extension<UserContext>,
) -> Result<Json<Vec<(String, DataType)>>, StatusCode> {
    match sqlx::query(r#"SELECT * FROM dynamic_fields"#)
        .fetch_all(&*user.db_pool)
        .await
    {
        Ok(vec) => {
            return Ok(Json(
                vec.into_iter()
                    .map(|row: PgRow| {
                        let column_name: String = row.get("name");
                        let data_type: &str = row.get("type");
                        match data_type {
                            "text" => (column_name, DataType::String),
                            "integer" => (column_name, DataType::Integer),
                            "float" => (column_name, DataType::Float),
                            "bool" => (column_name, DataType::Bool),
                            "paragraph" => (column_name, DataType::Text),
                            "percentage" => (column_name, DataType::Percentage),
                            "gallery" => (column_name, DataType::Gallery),
                            _ => {
                                if data_type.starts_with("list,") {
                                    let string_vec: Vec<String> = data_type
                                        .trim()
                                        .split(",")
                                        .skip(1)
                                        .map(|x| String::from(x))
                                        .collect();
                                    (column_name, DataType::List(string_vec))
                                } else {
                                    (column_name, DataType::String)
                                }
                            }
                        }
                    })
                    .collect(),
            ));
        }
        Err(e) => {
            println!("Get item column types error {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

async fn get_all_slaves(
    Extension(user): Extension<UserContext>,
    Json(payload): Json<IdRequest>,
) -> Result<Json<Vec<Container>>, StatusCode> {
    println!("in get all slaves");
    match sqlx::query(
        r#"
        SELECT slaves FROM containers WHERE id=$1
        "#,
    )
    .bind(payload.id)
    .fetch_one(&*user.db_pool)
    .await
    {
        Ok(slaves) => {
            let slave_string: String = slaves.get("slaves");
            let slave_ids: Vec<&str> = slave_string.split("@").filter(|x| !x.is_empty()).collect();
            match sqlx::query(
                r#"
                SELECT *
                FROM containers
                WHERE id = ANY($1)
                "#,
            )
            .bind(&slave_ids)
            .fetch_all(&*user.db_pool)
            .await
            {
                Ok(rows) => {
                    return Ok(Json(
                        rows.into_iter()
                            .map(|row: PgRow| {
                                let slaves: String = row.get("slaves");
                                let corners: String = row.get("corners");
                                let contained_items: String = row.get("contained_items");
                                return Container {
                                    id: row.get("id"),
                                    master: row.get("master"),
                                    slaves: string_to_set(&slaves),
                                    name: row.get("name"),
                                    corners: string_to_corners(&corners),
                                    image_type: row.get("image_type"),
                                    contained_items: string_to_set(&contained_items),
                                };
                            })
                            .collect(),
                    ));
                }
                Err(e) => {
                    println!("Get all slaves error 2 {}", e);
                    return Err(StatusCode::INTERNAL_SERVER_ERROR);
                }
            }
        }
        Err(e) => {
            println!("Get all slaves error 1 {}", e);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }
}

async fn remove_image(user_id: &str, item_id: &str) {
    let file_extensions = ["jpeg", "jpg", "png", "webp", "gif"];
    for ext in file_extensions {
        let _ = fs::remove_file(format!("/app/users/{}/images/{}.{}", user_id, item_id, ext)).await;
        let _ = fs::remove_file(format!(
            "/app/users/{}/images/small/{}.{}",
            user_id, item_id, ext
        ))
        .await;
        let _ = fs::remove_file(format!(
            "/app/users/{}/images/medium/{}.{}",
            user_id, item_id, ext
        ))
        .await;
    }
}

fn save_resized_images(
    data: &Bytes,
    user_id: &str,
    item_id: &str,
    file_extension: &str,
) -> Result<(), (StatusCode, String)> {
    println!("In upload image saved ok");
    //Figure out the dimensions
    let large_image = match image::load_from_memory(&data)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("Invalid image: {}", e)))
    {
        Ok(large_image) => large_image,
        Err(e) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error when loading image in save_resized_image: {:?}", e),
            ));
        }
    };
    let small_side: u32 = 150;
    let medium_side: u32 = 400;
    let (m_width, m_height);
    let (s_width, s_height);
    if large_image.height() > large_image.width() {
        (m_width, m_height) = (
            large_image.width() * medium_side / large_image.height(),
            medium_side,
        );
        (s_width, s_height) = (
            large_image.width() * small_side / large_image.height(),
            small_side,
        );
    } else {
        (m_width, m_height) = (
            medium_side,
            medium_side * large_image.height() / large_image.width(),
        );
        (s_width, s_height) = (
            small_side,
            small_side * large_image.height() / large_image.width(),
        )
    };
    let small_path = format!(
        "/app/users/{}/images/small/{}.{}",
        user_id, item_id, file_extension
    );
    let medium_path = format!(
        "/app/users/{}/images/medium/{}.{}",
        user_id, item_id, file_extension
    );
    //Resize and save
    //Pngs require rgba, webps can't handle rgba so they need to be handled differently
    if file_extension != "png" {
        match image::imageops::resize(
            &large_image.to_rgb8(),
            s_width,
            s_height,
            image::imageops::FilterType::Triangle,
        )
        .save(&small_path)
        {
            Ok(_) => {
                println!("Small image saved ok");
            }
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error when saving small: {}", e),
                ));
            }
        }
        match image::imageops::resize(
            &large_image.to_rgb8(),
            m_width,
            m_height,
            image::imageops::FilterType::Triangle,
        )
        .save(&medium_path)
        {
            Ok(_) => {
                println!("Medium saved ok");
                Ok(())
            }
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error when saving medium: {}", e),
                ));
            }
        }
    } else {
        match image::imageops::resize(
            &large_image,
            s_width,
            s_height,
            image::imageops::FilterType::Triangle,
        )
        .save(&small_path)
        {
            Ok(_) => {
                println!("Small image saved ok");
            }
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error when saving small: {}", e),
                ));
            }
        }
        match image::imageops::resize(
            &large_image,
            m_width,
            m_height,
            image::imageops::FilterType::Triangle,
        )
        .save(&medium_path)
        {
            Ok(_) => {
                println!("Medium saved ok");
                Ok(())
            }
            Err(e) => {
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error when saving medium: {}", e),
                ));
            }
        }
    }
}

async fn add_image_to_database(
    data: &Bytes,
    user: &UserContext,
    item_id: &str,
    file_extension: &str,
) -> Result<(), (StatusCode, String)> {
    println!("In upload image some file ext ok");
    if (infer::image::is_jpeg(&data) || infer::image::is_png(&data) || infer::image::is_webp(&data))
        && !item_id.is_empty()
    {
        println!("In upload image inner ok");
        //Update the database, if the id doesn't exist in the containers, try the items
        let container_update = sqlx::query(
            r#"
            UPDATE containers SET image_type=$1 WHERE id=$2
            "#,
        )
        .bind(&file_extension)
        .bind(&item_id)
        .execute(&*user.db_pool)
        .await;

        let mut container_found = false;
        if let Ok(container_update) = container_update {
            if container_update.rows_affected() == 1 {
                println!(
                    "container rows affected: {}",
                    container_update.rows_affected()
                );
                container_found = true;
            }
        }

        if !container_found {
            let item_update = sqlx::query(
                r#"
                           UPDATE items SET image_type=$1 WHERE id=$2
                           "#,
            )
            .bind(&file_extension)
            .bind(&item_id)
            .execute(&*user.db_pool)
            .await;
            match item_update {
                Ok(item_update) => {
                    if item_update.rows_affected() != 1 {
                        println!("item 1 rows affected: {}", item_update.rows_affected());
                        return Err((
                            StatusCode::INTERNAL_SERVER_ERROR,
                            "Id not in items".to_string(),
                        ));
                    }
                }
                Err(e) => {
                    println!("item table update error: {}", e);
                    return Err((
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Id not in items2".to_string(),
                    ));
                }
            }
        }
        //Check for an existing image with the same id and if so, delete it
        remove_image(&user.user_id, &item_id).await;
        //Now that the file is a valid image and the id is in the database, save it
        // Create the full path for saving the file
        let path = format!(
            "/app/users/{}/images/{}.{}",
            user.user_id, item_id, file_extension
        );
        // Save the file
        return match fs::write(&path, &data).await {
            Ok(_) => match save_resized_images(&data, &user.user_id, &item_id, &file_extension) {
                Ok(_) => Ok(()),
                Err(e) => Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to save smaller files: {:?}", e),
                )),
            },
            Err(e) => {
                println!("Error when saving: {}", e);
                return Err((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to save file: {}", e),
                ));
            }
        };
    }
    return Err((
        StatusCode::INTERNAL_SERVER_ERROR,
        "No image file found in request".to_string(),
    ));
}

async fn upload_image(
    Extension(user): Extension<UserContext>,
    mut multipart: Multipart,
) -> Result<(), (StatusCode, String)> {
    println!("In upload image");
    // Process the multipart form data
    let mut data: Option<axum::body::Bytes> = None;
    let mut file_extension: Option<String> = None;
    let mut item_id: Option<String> = None;
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        println!("Upload image error {}", e);
        (StatusCode::BAD_REQUEST, e.to_string())
    })? {
        if let Some(name) = field.name() {
            match name {
                "image" => {
                    match field.file_name() {
                        Some(filename) => {
                            let filename: Vec<&str> = filename.split(".").collect();
                            if filename.len() == 2 {
                                file_extension = Some(filename[1].to_owned());
                            } else {
                                return Err((
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    "Failed to read file name".to_string(),
                                ));
                            }
                        }
                        None => {
                            return Err((
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to read file name".to_string(),
                            ));
                        }
                    }
                    if let Some(content_type) = field.content_type() {
                        // Check if it's an image type
                        if content_type == ("image/jpeg")
                            || content_type == ("image/png")
                            || content_type == ("image/webp")
                            || content_type == ("image/gif")
                        {
                            data = Some(field.bytes().await.map_err(|e| {
                                println!("Failed to read file data");
                                (
                                    StatusCode::INTERNAL_SERVER_ERROR,
                                    format!("Failed to read file data : {}", e),
                                )
                            })?);
                        }
                    }
                }
                "item_id" => {
                    item_id = Some(field.text().await.map_err(|e| {
                        println!("Failed to read file id");
                        (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            format!("Failed to read file id: {}", e),
                        )
                    })?);
                }
                _ => (),
            }
        }
    }
    println!("In upload image read data ok");
    if let Some(data) = data {
        if let Some(file_extension) = file_extension {
            if let Some(item_id) = item_id {
                return add_image_to_database(&data, &user, &item_id, &file_extension).await;
            }
        }
    }
    Err((
        StatusCode::BAD_REQUEST,
        "No file found in request".to_string(),
    ))
}

#[derive(Deserialize, PartialEq)]
enum ImageSize {
    Small,
    Medium,
    Large,
}

#[derive(Deserialize)]
struct ImageRequest {
    image_id: String,
    image_type: String,
    image_size: ImageSize,
}

async fn serve_image(
    Extension(user): Extension<UserContext>,
    Json(payload): Json<ImageRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    // Construct the file path using the authenticated user's ID
    println!("Gets in serve image");

    let file_path: String = if payload.image_size == ImageSize::Large {
        format!(
            "/app/users/{}/images/{}.{}",
            user.user_id, payload.image_id, payload.image_type
        )
    } else if payload.image_size == ImageSize::Medium {
        format!(
            "/app/users/{}/images/medium/{}.{}",
            user.user_id, payload.image_id, payload.image_type
        )
    } else {
        format!(
            "/app/users/{}/images/small/{}.{}",
            user.user_id, payload.image_id, payload.image_type
        )
    };
    println!("{}", file_path);

    // Try to open the file asynchronously
    let file = File::open(&file_path)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;
    println!("Found the file");
    let stream = ReaderStream::new(file);
    let body = axum::body::Body::from_stream(stream);
    // Determine the content type based on the file extension (basic example)
    let content_type = match payload.image_type.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => return Err(StatusCode::BAD_REQUEST), // Fallback for unknown types
    };

    println!("Loaded the body and headers");
    // Build the response with headers and body
    let response = axum::http::Response::builder()
        .header(axum::http::header::CONTENT_TYPE, content_type)
        .body(body)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(response)
}
