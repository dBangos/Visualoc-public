use database::{async_db::send_command_to_database, data_helpers::ImageSize};
use egui::{
    Color32, ColorImage, FontFamily, FontId, Style, TextStyle, TextWrapMode, TextureHandle,
    Visuals,
    ahash::{HashMap, HashMapExt},
};
use rfd::FileHandle;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet};
use uuid::Uuid;
use web_sys::FileList;
mod gui {
    pub mod account;
    pub mod canvas;
    pub mod edit_location;
    pub mod fields_page;
    pub mod gui_helpers;
    pub mod home_page;
    pub mod locations_page;
    pub mod login;
    pub mod modal;
    pub mod statistics;
    pub mod top_row;
}

mod database {
    pub mod async_db;
    pub mod containers;
    pub mod data_helpers;
    pub mod items;
}

#[derive(PartialEq)]
enum ModalType {
    None,
    DeleteItem,
    DeleteLocation,
    DeleteContainer,
    DeleteField,
    RemoveFromContainer,
    SelectContainerlessItem,
    SelectFieldsShown,
    ItemImage,
    Backup,
    Settings,
    AddLocation,
}

enum UIPages {
    Home,
    LocationGrid,
    LocationContainers,
    Statistics,
    Account,
}

#[derive(PartialEq)]
enum ContainerScreen {
    None,
    AddingContainer,
    SelectedContainer,
    EditingContainer,
    AddingItem,
    SelectedItem,
    ItemNotInContainer,
    EditingItem,
    EditingLocation,
}

#[derive(PartialEq)]
enum FieldModal {
    None,
    Start,
    AddingField,
    DeletingField,
    EditingField(bool),
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
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

//On all commands the first string is the command id
#[derive(Debug)]
enum CommandToServer {
    RegisterUser(String, RegisterResult, String, String, String),
    LoginUser(String, LoginResult, String, String),
    Authenticate(String, String, LoginResult),

    AddContainer(String, Container),
    UpdateContainer(String, Container),
    DeleteContainer(String, Container),

    UpdateItem(String, ContainedItem, Vec<(String, DataType)>),
    InsertItem(String, ContainedItem, String, Vec<(String, DataType)>),
    DeleteItem(String, ContainedItem, String, bool),
    SearchItems(String, String, String, Vec<ContainedItem>),

    GetItemColumnTypes(String, Vec<(String, DataType)>),
    GetAllSlaves(String, String, Vec<Container>),
    GetItemLocationContainer(String, String, Option<(Container, Container)>),
    GetMultipleItems(String, BTreeSet<String>, Vec<ContainedItem>),
    GetAllItemIdsNotInContainer(String, Vec<String>),

    AddField(String, String, DataType),
    DeleteColumnFromItems(String, String),
    UpdateItemsColumn(String, (String, DataType), String),

    GetImageFromServer(String, String, String, ImageSize, ColorImage),
    AddImage(String, String, String),
    GetBackup(String),

    PickBackupDumpFile(String, Option<FileHandle>),
    PickBackupImageFolder(String, Option<FileList>),
    UploadBackup(String, Option<FileHandle>, Option<FileList>),
}

#[derive(Clone)]
enum WaitingFunctionKind {
    LoadLocationsPage,
    AddFieldOk,
    DeleteFieldOk,
    DeleteContainerOk1,
    DeleteContainerOk2,
    AddExistingItemClicked1,
    AddExistingItemClicked2,
}

#[derive(PartialEq, Debug, Deserialize, Clone, Copy)]
enum RegisterResult {
    None,
    Created,
    Error,
    UsernameInUse,
    EmailInUse,
    InvalidPassword,
    InvalidEmail,
}

#[derive(PartialEq, Debug, Deserialize, Clone)]
enum LoginResult {
    Success(String),
    Error,
    None,
    UsernameDoesntExist,
    WrongPassword,
}

enum BackupState {
    Start,
    Upload,
    Waiting,
}

struct HomePage {
    page_number: usize,
    previous_search: String, //String to compare to, to see if search has to be run again
    column_search: (String, DataType),
    previous_column_search: String,
    search_results_sorted: bool,
}

impl Default for HomePage {
    fn default() -> Self {
        Self {
            page_number: 0,
            previous_search: String::new(),
            column_search: ("Name".to_string(), DataType::String),
            previous_column_search: String::new(),
            search_results_sorted: false,
        }
    }
}

struct ModalVars {
    field_modal: FieldModal,
    item_field_selected_fields: Vec<bool>,
    new_field_name: String,
    new_field_type: DataType,
    modal_type: ModalType,
    modal_id: String, //Different id for every modal spawned
    field_modal_id: String,
}

impl Default for ModalVars {
    fn default() -> Self {
        Self {
            modal_id: Uuid::new_v4().to_string(),
            field_modal: FieldModal::None,
            item_field_selected_fields: Vec::new(),
            new_field_name: String::new(),
            new_field_type: DataType::String,
            modal_type: ModalType::None,
            field_modal_id: Uuid::new_v4().to_string(),
        }
    }
}

struct Login {
    username_string: String,
    password_string: String,
    register_page: bool,
    email_string: String,
    password_confirmation_string: String,
    register_result: RegisterResult,
    login_result: LoginResult,
    session_token: String,
    show_password: bool,
    automatic_login_attempted: bool,
}

impl Default for Login {
    fn default() -> Self {
        Self {
            username_string: String::new(),
            password_string: String::new(),
            email_string: String::new(),
            register_page: false,
            register_result: RegisterResult::None,
            login_result: LoginResult::None,
            password_confirmation_string: String::new(),
            session_token: String::new(),
            show_password: false,
            automatic_login_attempted: false,
        }
    }
}

struct Backup {
    state: BackupState,
    dump_filehandle: Option<FileHandle>,
    images_filehandle: Option<FileList>,
    resulting_string: String,
}

impl Default for Backup {
    fn default() -> Self {
        Self {
            state: BackupState::Start,
            dump_filehandle: None,
            images_filehandle: None,
            resulting_string: String::new(),
        }
    }
}

#[derive(Clone)]
struct WaitingFunction {
    id: String,
    kind: WaitingFunctionKind,
}

#[derive(Clone, Serialize, Deserialize)]
struct Settings {
    light_mode: bool,
    items_per_page: usize,
    show_rectangles: bool,
    rectangle_opacity: f32,
    rectangle_colour: Color32,
    selected_rectangle_colour: Color32,
    border_colour: Color32,
    show_container_names: bool,
    container_name_colour: Color32,
    ui_scale_temp: f32,
    ui_scale: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            light_mode: false,
            items_per_page: 12,
            show_rectangles: true,
            rectangle_opacity: 0.5,
            rectangle_colour: Color32::WHITE,
            selected_rectangle_colour: Color32::RED,
            border_colour: Color32::BLACK,
            show_container_names: true,
            container_name_colour: Color32::BLACK,
            ui_scale_temp: 1.0,
            ui_scale: 1.0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Container {
    id: String,
    master: String,
    slaves: BTreeSet<String>,
    name: String,
    corners: [f32; 4],
    image_type: String,
    contained_items: BTreeSet<String>,
}

impl Default for Container {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name: "".to_string(),
            master: "".to_string(),
            slaves: BTreeSet::new(),
            corners: [0.0, 0.0, 0.0, 0.0],
            image_type: String::new(),
            contained_items: BTreeSet::new(),
        }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)] //If the old state is missing fields(eg when added in a new version) replace them with their defaults
struct Visualoc {
    #[serde(skip)]
    initialized: bool,
    #[serde(skip)]
    host: String,
    #[serde(skip)]
    current_ui: UIPages,
    #[serde(skip)]
    container_screen: ContainerScreen,
    #[serde(skip)]
    source_node_id: String,
    #[serde(skip)]
    selected_location: Container,
    #[serde(skip)]
    selected_container: Container,
    #[serde(skip)]
    selected_item: ContainedItem,
    #[serde(skip)]
    container_vec: Vec<Container>,
    #[serde(skip)]
    item_vec: Vec<ContainedItem>,
    #[serde(skip)]
    item_page_search_vec: Vec<ContainedItem>,
    #[serde(skip)]
    drag_started_on_canvas: bool,
    #[serde(skip)]
    redraw_canvas_image: bool,
    #[serde(skip)]
    loaded_images: HashMap<String, Option<(TextureHandle, ImageSize)>>,
    #[serde(skip)]
    login: Login, //All the variables for the login screen
    #[serde(skip)]
    modal_vars: ModalVars,
    #[serde(skip)]
    item_field_types: Vec<(String, DataType)>,
    #[serde(skip)]
    search_string: String,
    //=========================================
    //Database
    //=========================================
    #[serde(skip)]
    tokio_sender: tokio::sync::mpsc::Sender<CommandToServer>,
    #[serde(skip)]
    tokio_receiver: tokio::sync::mpsc::Receiver<CommandToServer>,
    #[serde(skip)]
    async_tasks_to_send: Vec<CommandToServer>,
    #[serde(skip)]
    async_tasks_sent_ids: HashSet<String>,
    #[serde(skip)]
    functions_waiting_data: Vec<WaitingFunction>,
    //=========================================
    //Start page variables
    //=========================================
    #[serde(skip)]
    home_page: HomePage,
    //=========================================
    //Values for the add existing item modal
    //=========================================
    #[serde(skip)]
    containerless_items: Vec<ContainedItem>,
    #[serde(skip)]
    containerless_items_bools: Vec<bool>,
    #[serde(skip)]
    containerless_items_ids: BTreeSet<String>,
    //=========================================
    //Location Grid settings
    //=========================================
    #[serde(skip)]
    new_ordered_locations_vec: Vec<String>, //Vec to hold the new values when user is editing the order
    #[serde(skip)]
    rearrange_locations: bool,
    //=========================================
    //UI settings
    //=========================================
    #[serde(skip)]
    temp_settings: Settings,
    #[serde(skip)]
    backup: Backup,
    //=========================================
    //Statistics
    //=========================================
    #[serde(skip)]
    item_count: usize,
    #[serde(skip)]
    location_count: usize,
    #[serde(skip)]
    max_min_field_values: Vec<Vec<(String, usize)>>,
    //=========================================
    //Persistent variables
    //=========================================
    settings: Settings,
    show_all_fields: bool,              //Home page
    item_fields_shown: Vec<bool>,       //Home page which columns are shown
    ordered_locations_vec: Vec<String>, //Locations page ID's of the locations in a user defined order
    remember_login: bool,               //Login
    persistent_token: String,           //Login
}

impl Default for Visualoc {
    fn default() -> Self {
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        Self {
            initialized: false,
            host: "https://api.visualoc.com/".to_owned(),
            // host: "http://localhost:8010/".to_owned(),
            current_ui: UIPages::Home,
            container_screen: ContainerScreen::None,
            source_node_id: "Source".to_string(),
            selected_location: Container::default(),
            selected_container: Container::default(),
            selected_item: ContainedItem::default(),
            loaded_images: HashMap::new(),
            login: Login::default(),
            remember_login: false,
            persistent_token: String::new(),
            modal_vars: ModalVars::default(),
            home_page: HomePage::default(),
            item_field_types: Vec::new(),
            search_string: String::new(),
            tokio_sender: tx,
            tokio_receiver: rx,
            async_tasks_to_send: Vec::new(),
            async_tasks_sent_ids: HashSet::new(),
            functions_waiting_data: Vec::new(),
            container_vec: Vec::new(),
            item_vec: Vec::new(),
            item_page_search_vec: Vec::new(),
            drag_started_on_canvas: false,
            redraw_canvas_image: false,
            show_all_fields: true,
            item_fields_shown: Vec::new(),
            containerless_items: Vec::new(),
            containerless_items_bools: Vec::new(),
            containerless_items_ids: BTreeSet::new(),
            ordered_locations_vec: Vec::new(),
            new_ordered_locations_vec: Vec::new(),
            rearrange_locations: false,
            settings: Settings::default(),
            temp_settings: Settings::default(),
            backup: Backup::default(),
            item_count: 0,
            location_count: 0,
            max_min_field_values: Vec::new(),
        }
    }
}

impl Visualoc {
    //Function used during initialization
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        //Configure the style of the app
        cc.egui_ctx.set_style(configure_style());
        //If there is a state saved return that
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        //Else return the default state
        return Default::default();
    }
}

fn configure_style() -> Style {
    let new_text_size = 20.0;
    return egui::Style {
        text_styles: [
            (
                TextStyle::Heading,
                FontId::new(new_text_size * 1.2, FontFamily::Proportional),
            ),
            (
                TextStyle::Body,
                FontId::new(new_text_size, FontFamily::Proportional),
            ),
            (
                TextStyle::Button,
                FontId::new(new_text_size, FontFamily::Proportional),
            ),
            (
                TextStyle::Monospace,
                FontId::new(new_text_size, FontFamily::Monospace),
            ),
            (
                TextStyle::Small,
                FontId::new(new_text_size, FontFamily::Proportional),
            ),
        ]
        .into(),
        wrap_mode: Some(TextWrapMode::Truncate),

        ..Default::default()
    };
}

fn toggle_light_mode(ctx: &egui::Context, toggle: bool) {
    if toggle {
        let mut custom_light = Visuals::light();
        custom_light.panel_fill = Color32::from_rgb(230, 230, 230);
        custom_light.widgets.inactive.weak_bg_fill = Color32::from_rgb(200, 200, 200);
        custom_light.widgets.inactive.bg_fill = Color32::from_rgb(255, 255, 255);
        ctx.set_visuals(custom_light);
    } else {
        ctx.set_visuals(Visuals::dark());
    }
}

// When compiling natively:
#[cfg(not(target_arch = "wasm32"))]
fn main() -> Result<(), eframe::Error> {
    //Create the images directory
    let _ = std::fs::create_dir("images");
    // let icon = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icon.png")).unwrap();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1820.0, 1000.0]),
        ..Default::default()
    };
    eframe::run_native(
        "Visualoc",
        options,
        Box::new(|cc| {
            // This gives us image support:
            egui_extras::install_image_loaders(&cc.egui_ctx);
            //Run new to get the initial visualoc object
            Ok(Box::new(Visualoc::new(cc)))
        }),
    )
}

// When compiling to web using trunk:
#[cfg(target_arch = "wasm32")]
fn main() {
    use eframe::wasm_bindgen::JsCast as _;
    // Redirect `log` message to `console.log` and friends:
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();
    let web_options = eframe::WebOptions::default();
    wasm_bindgen_futures::spawn_local(async {
        let document = web_sys::window()
            .expect("No window")
            .document()
            .expect("No document");
        let canvas = document
            .get_element_by_id("the_canvas_id")
            .expect("Failed to find the_canvas_id")
            .dyn_into::<web_sys::HtmlCanvasElement>()
            .expect("the_canvas_id was not a HtmlCanvasElement");
        let start_result = eframe::WebRunner::new()
            .start(
                canvas,
                web_options,
                Box::new(|cc| {
                    // This gives us image support:
                    egui_extras::install_image_loaders(&cc.egui_ctx);
                    //Run new to get the initial visualoc object
                    Ok(Box::new(Visualoc::new(cc)))
                }),
            )
            .await;

        // Remove the loading text and spinner:
        if let Some(loading_text) = document.get_element_by_id("loading_text") {
            match start_result {
                Ok(_) => {
                    loading_text.remove();
                }
                Err(e) => {
                    loading_text.set_inner_html(
                        "<p> The app has crashed. See the developer console for details. </p>",
                    );
                    panic!("Failed to start eframe: {e:?}");
                }
            }
        }
    });
}

impl eframe::App for Visualoc {
    //Save the state before shutdown
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    //Update on every frame
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        //Initialization logic
        ctx.set_pixels_per_point(self.settings.ui_scale);
        if self.remember_login
            && !self.persistent_token.is_empty()
            && self.login.session_token.is_empty()
            && !self.login.automatic_login_attempted
        {
            self.login.automatic_login_attempted = true;
            self.async_tasks_to_send.push(CommandToServer::Authenticate(
                Uuid::new_v4().to_string(),
                self.persistent_token.clone(),
                LoginResult::None,
            ));
        } else if self.login.session_token.is_empty() {
            egui::CentralPanel::default().show(ctx, |ui| {
                self.login_page(ui);
            });
        } else {
            //=================================================================
            if !self.initialized {
                self.initialize(ctx);
                self.initialized = true;
            }
            //=================================================================
            if self.initialized {
                //Execute whatever functions need data to proceed
                self.execute_waiting_functions();
                egui::CentralPanel::default().show(ctx, |ui| {
                    egui::TopBottomPanel::top("top_panel")
                        .resizable(false)
                        .min_height(32.0)
                        .show_inside(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                self.top_row(ui);
                            });
                        });
                    egui::CentralPanel::default().show_inside(ui, |ui| match self.current_ui {
                        UIPages::Home => self.home_page(ui, ctx),
                        UIPages::LocationGrid => self.location_grid_screen(ui),
                        UIPages::LocationContainers => self.location_containers_screen(ui, ctx),
                        UIPages::Statistics => self.statistics_screen(ui),
                        UIPages::Account => self.account_page(ui),
                    });
                    if self.modal_vars.modal_type != ModalType::None {
                        //Pass a new uuid
                        self.spawn_modal(ctx, self.modal_vars.modal_id.clone());
                    }
                    if self.modal_vars.field_modal != FieldModal::None {
                        self.dynamic_field_edit_modal(ctx);
                    }
                });
            }
        }
        //Sends all the accumulated commands to the database
        send_command_to_database(
            &self.tokio_sender,
            &mut self.async_tasks_to_send,
            &mut self.async_tasks_sent_ids,
            &self.host,
            &self.login.session_token,
        );
        //Checks the channel from the database for completion
        self.parse_command(ctx);
        ctx.request_repaint();
    }
}
