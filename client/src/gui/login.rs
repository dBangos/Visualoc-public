use egui::Color32;
use log::{Level, log};
use reqwest::Client;
use serde::Serialize;
use tokio::sync::mpsc::Sender;
use uuid::Uuid;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;
use web_sys::{Document, HtmlFormElement, HtmlInputElement, Window};

use crate::{CommandToServer, LoginResult, RegisterResult, Visualoc};

#[derive(Serialize)]
struct RegisterRequest {
    username: String,
    password: String,
    email: String,
}

#[derive(Serialize)]
struct LoginRequest {
    username: String,
    password: String,
}

impl Visualoc {
    fn update_dom_username_password(&self) {
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

        match document.get_element_by_id("username") {
            Some(element) => match element.dyn_into::<HtmlInputElement>() {
                Ok(username) => {
                    username.set_value(&self.login.username_string);
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

        match document.get_element_by_id("password") {
            Some(element) => match element.dyn_into::<HtmlInputElement>() {
                Ok(password) => {
                    password.set_value(&self.login.password_string);
                }
                Err(e) => {
                    log::log!(Level::Error, "Element dyn into error 2: {:?}", e);
                    return;
                }
            },
            None => {
                log::log!(Level::Error, "Document get element folder_input error 2");
                return;
            }
        };
    }

    fn submit_credentials_form(&self) {
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

        match document.get_element_by_id("credentials_form") {
            Some(form) => match form.dyn_into::<HtmlFormElement>() {
                Ok(form) => {
                    let _ = form.submit();
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
    }

    pub fn login_page(&mut self, ui: &mut egui::Ui) {
        if self.login.register_result == RegisterResult::Created {
            self.login.register_page = false;
            self.login.show_password = false;
        }
        if !self.login.register_page {
            ui.vertical_centered(|ui| {
                match self.login.login_result.clone() {
                    LoginResult::None => (),
                    LoginResult::Error => {
                        ui.colored_label(
                            Color32::RED,
                            "There was an error when logging in, please try again later",
                        );
                    }
                    LoginResult::WrongPassword => {
                        ui.colored_label(Color32::RED, "Wrong password");
                    }
                    LoginResult::UsernameDoesntExist => {
                        ui.colored_label(Color32::RED, "There is no user by that username");
                    }
                    LoginResult::Success(temp) => {
                        self.login.session_token = temp.clone();
                        if self.remember_login {
                            self.persistent_token = temp.clone();
                        }
                    }
                }
                if self.login.register_result == RegisterResult::Created {
                    ui.label("Your account has been created, you can now log in");
                    ui.add_space(15.0);
                }

                ui.label("Login");
                ui.horizontal(|ui| {
                    let text_size =
                        Visualoc::calculate_label_size_with_wrap(ui, "Username:", 1000.0);
                    ui.add_space(ui.available_size().x / 2.0 - 100.0 - text_size.width());
                    ui.label("Username:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.login.username_string)
                            .hint_text("Username"),
                    );
                });
                ui.horizontal(|ui| {
                    let text_size =
                        Visualoc::calculate_label_size_with_wrap(ui, "Password:", 1000.0);
                    ui.add_space(ui.available_size().x / 2.0 - 100.0 - text_size.width());
                    ui.label("Password:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.login.password_string)
                            .hint_text("Password")
                            .password(!self.login.show_password),
                    );
                });
                self.update_dom_username_password();
                ui.checkbox(&mut self.login.show_password, "Show password");
                ui.checkbox(&mut self.remember_login, "Remember me on this device");
                if ui.button("Login").clicked() {
                    self.persistent_token = String::new();
                    self.async_tasks_to_send.push(CommandToServer::LoginUser(
                        Uuid::new_v4().to_string(),
                        crate::LoginResult::Error,
                        self.login.username_string.clone(),
                        self.login.password_string.clone(),
                    ));
                    self.submit_credentials_form();
                }
                if ui.button("Register").clicked() {
                    self.login.register_page = true;
                    self.login.login_result = LoginResult::None;
                }
            });
        } else {
            ui.vertical_centered(|ui| {
                if self.login.register_result == RegisterResult::EmailInUse {
                    ui.colored_label(Color32::RED, "There is already an account with this email");
                }
                if self.login.register_result == RegisterResult::UsernameInUse {
                    ui.colored_label(
                        Color32::RED,
                        "There is already an account with this username",
                    );
                }
                if self.login.register_result == RegisterResult::InvalidPassword {
                    ui.colored_label(
                        Color32::RED,
                        "Your password has to be over 6 characters long and contain a number",
                    );
                }
                if self.login.register_result == RegisterResult::InvalidEmail {
                    ui.colored_label(Color32::RED, "Please use a valid email address");
                }

                ui.label("Create Account");
                ui.horizontal(|ui| {
                    let text_size =
                        Visualoc::calculate_label_size_with_wrap(ui, "Username:", 1000.0);
                    ui.add_space(ui.available_size().x / 2.0 - 100.0 - text_size.width());
                    ui.label("Username:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.login.username_string)
                            .hint_text("Username"),
                    );
                });
                ui.horizontal(|ui| {
                    let text_size =
                        Visualoc::calculate_label_size_with_wrap(ui, "Password:", 1000.0);
                    ui.add_space(ui.available_size().x / 2.0 - 100.0 - text_size.width());
                    ui.label("Password:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.login.password_string)
                            .hint_text("Password")
                            .password(!self.login.show_password),
                    );
                });
                ui.horizontal(|ui| {
                    let text_size =
                        Visualoc::calculate_label_size_with_wrap(ui, "Confirm Password:", 1000.0);
                    ui.add_space(ui.available_size().x / 2.0 - 100.0 - text_size.width());
                    ui.label("Confirm Password:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.login.password_confirmation_string)
                            .hint_text("Confirm Password")
                            .password(!self.login.show_password),
                    );
                });
                ui.checkbox(&mut self.login.show_password, "Show password");
                if self.login.password_string != self.login.password_confirmation_string {
                    ui.colored_label(Color32::RED, "Passwords do not match");
                }
                ui.horizontal(|ui| {
                    let text_size = Visualoc::calculate_label_size_with_wrap(ui, "Email:", 1000.0);
                    ui.add_space(ui.available_size().x / 2.0 - 100.0 - text_size.width());
                    ui.label("Email:");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.login.email_string).hint_text("Email"),
                    );
                });
                if ui.button("Register").clicked()
                    && self.login.password_string == self.login.password_confirmation_string
                    && self.login.password_string != String::default()
                {
                    self.async_tasks_to_send.push(CommandToServer::RegisterUser(
                        Uuid::new_v4().to_string(),
                        RegisterResult::None,
                        self.login.username_string.to_owned(),
                        self.login.password_string.to_owned(),
                        self.login.email_string.to_owned(),
                    ));
                }
                if ui.button("Back").clicked() {
                    self.login.register_page = false;
                    self.login.show_password = false;
                }
            });
        }
    }
}

pub fn register_user_request(
    host: &str,
    tx: &Sender<CommandToServer>,
    id: &str,
    username: &str,
    password: &str,
    email: &str,
) {
    let id = id.to_owned();
    let host = host.to_owned();
    let tx = tx.clone();

    let request_data = RegisterRequest {
        password: password.to_string(),
        email: email.to_string(),
        username: username.to_string(),
    };
    spawn_local(async move {
        let response = Client::new()
            .post(host + "register")
            .json(&request_data)
            .send()
            .await;
        match response {
            Ok(resp) => match resp.json().await {
                Ok(res) => {
                    if let Err(e) = tx
                        .send(CommandToServer::RegisterUser(
                            id,
                            res,
                            String::default(),
                            String::default(),
                            String::default(),
                        ))
                        .await
                    {
                        println!("Error when sending the register_user back: {}", e);
                    }
                }
                Err(e) => println!("Error when deserializing json register user: {}", e),
            },

            Err(e) => println!("Register user types error {}", e),
        }
    });
}

pub fn login_user_request(
    host: &str,
    tx: &Sender<CommandToServer>,
    id: &str,
    username: &str,
    password: &str,
) {
    let id = id.to_owned();
    let host = host.to_owned();
    let tx = tx.clone();

    let request_data = LoginRequest {
        password: password.to_string(),
        username: username.to_string(),
    };
    log!(
        Level::Info,
        "In login with data: {:?} {}",
        request_data.password.clone(),
        request_data.username.clone()
    );
    spawn_local(async move {
        let response = Client::new()
            .post(host + "login")
            .json(&request_data)
            .send()
            .await;
        match response {
            Ok(resp) => match resp.json().await {
                Ok(login_response) => {
                    if let Err(e) = tx
                        .send(CommandToServer::LoginUser(
                            id,
                            login_response,
                            String::default(),
                            String::default(),
                        ))
                        .await
                    {
                        log!(
                            Level::Error,
                            "Error when sending the login_user back: {}",
                            e
                        );
                        println!("Error when sending the login_user back: {}", e);
                    }
                }
                Err(e) => {
                    log!(
                        Level::Error,
                        "Error when deserializing json login user: {}",
                        e
                    );
                    println!("Error when deserializing json login user: {}", e);
                }
            },

            Err(e) => {
                log!(Level::Error, "Login user types error {}", e);
                println!("Login user types error {}", e);
            }
        }
    });
}

pub fn initial_authentication(host: &str, tx: &Sender<CommandToServer>, cmd_id: &str, token: &str) {
    let host = host.to_owned();
    let token = token.to_owned();
    let cmd_id = cmd_id.to_owned();
    let tx = tx.clone();
    spawn_local(async move {
        match Client::new()
            .post(host + "authenticate")
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(response) => match response.json::<LoginResult>().await {
                Ok(login_response) => {
                    if let Err(e) = tx
                        .send(CommandToServer::Authenticate(
                            cmd_id,
                            String::default(),
                            login_response,
                        ))
                        .await
                    {
                        log!(
                            Level::Error,
                            "Error when sending the login_user back: {}",
                            e
                        );
                    }
                }
                Err(_) => {
                    if let Err(e) = tx
                        .send(CommandToServer::Authenticate(
                            cmd_id,
                            String::default(),
                            LoginResult::Error,
                        ))
                        .await
                    {
                        log!(
                            Level::Error,
                            "Error when sending the login_user back: {}",
                            e
                        );
                    }
                }
            },
            Err(_) => {
                if let Err(e) = tx
                    .send(CommandToServer::Authenticate(
                        cmd_id,
                        String::default(),
                        LoginResult::Error,
                    ))
                    .await
                {
                    log!(
                        Level::Error,
                        "Error when sending the login_user back: {}",
                        e
                    );
                }
            }
        }
    });
}
