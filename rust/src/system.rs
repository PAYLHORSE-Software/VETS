use std::io::ErrorKind::WouldBlock;
use std::thread;
use std::time::Duration;
use std::io::Cursor;
use std::sync::{Arc, Mutex, MutexGuard};
use godot::prelude::*;
use godot::classes::{InputEvent, Control, PanelContainer, VBoxContainer, HBoxContainer, Image, ImageTexture, TextureRect, LineEdit, TextEdit, RichTextLabel, FileAccess, OptionButton, CheckButton, Font};
use godot::classes::file_access::ModeFlags;
use xcap::Window;
use image::{ImageBuffer, Rgba, ImageOutputFormat, GenericImageView, DynamicImage};
use base64::encode;
use serde::{Serialize, Deserialize};
use serde_json::{json, Value};
use reqwest;
use kakasi::IsJapanese;
use crate::utils::*;
use crate::gui::sandGUI;

struct ScreenCapture {
    png_buffer: Cursor<Vec<u8>>,
    is_preview: bool,
}

struct ErrorOrWarning {
    string: String,
    is_warning: bool,
}

struct TranslationPacket {
    jp_text: String,
    jp_read: String,
    eng_text: String,
}

#[derive(Serialize, Deserialize)]
struct UserSettings {
    user_credentials: Option<UserCredentials>,
    reading_area: Option<ReadingArea>,
    packet_config: Option<PacketConfig>,
}

#[derive(Serialize, Deserialize)]
struct UserCredentials {
        gcloud_token: String,
        project_id: String,
        deepl_token: String,
}

#[derive(Serialize, Deserialize)]
struct ReadingArea {
    up: u32,
    down: u32,
    left: u32,
    right: u32,
}

#[derive(Serialize, Deserialize)]
struct PacketConfig {
    jp_font: i32,
    font_size: usize,
    romaji: bool,
}

enum SystemState {
    IDLE,
    CAPTURING,
    READING,
    PROCESSING,
}

#[derive(GodotClass)]
#[class(base = Node)]
pub struct System {
    #[base]
    pub node: Base<Node>,
    system_state: SystemState,
    time_accumulator: f32,
    is_preview: bool,
    screen_queue: Arc<Mutex<Vec<ScreenCapture>>>,
    packets_queue: Arc<Mutex<Vec<Vec<TranslationPacket>>>>,
    error_queue: Arc<Mutex<Vec<ErrorOrWarning>>>,
    box_queue: Arc<Mutex<Vec<()>>>,
}

#[godot_api]
impl INode for System {
    fn init(node: Base<Node>) -> Self {
        System {
            node,
            system_state: SystemState::IDLE,
            time_accumulator: 0.0,
            is_preview: false,
            screen_queue: Arc::new(Mutex::new(Vec::new())),
            packets_queue: Arc::new(Mutex::new(Vec::new())),
            error_queue: Arc::new(Mutex::new(Vec::new())),
            box_queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn ready(&mut self) {
        godot_print!("Starting VETS...");
        // -- MOUSE CURSOR --
        let mouse_cursor = load::<Resource>("res://menu/sprite/mouse_cursor.png");
        Input::singleton().set_custom_mouse_cursor(&mouse_cursor);
        // -- LOAD USER SETTINGS --
        if FileAccess::file_exists("user://user_settings.toml") {
            self.load_user_settings();
        }
        self.refresh_preview_packet();
        self.list_windows();
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        // --- MOUSE CURSOR ---
        let is_mouse_clicked = Input::singleton().is_action_just_pressed("mouse_click");
        let is_mouse_released = Input::singleton().is_action_just_released("mouse_click");
        let is_mouse_right_clicked = Input::singleton().is_action_just_pressed("mouse_rightclick");
        let is_mouse_right_released = Input::singleton().is_action_just_released("mouse_rightclick");
        let is_capture_pressed = Input::singleton().is_action_just_pressed("capture");
        if is_mouse_clicked {
            let mouse_cursor = load::<Resource>("res://menu/sprite/mouse_cursor_2.png");
            Input::singleton().set_custom_mouse_cursor(&mouse_cursor);
        }
        if is_mouse_released {
            let mouse_cursor = load::<Resource>("res://menu/sprite/mouse_cursor.png");
            Input::singleton().set_custom_mouse_cursor(&mouse_cursor);
        }
        if is_mouse_right_clicked {
            let mouse_cursor = load::<Resource>("res://menu/sprite/mouse_cursor_3.png");
            Input::singleton().set_custom_mouse_cursor(&mouse_cursor);
        }
        if is_mouse_right_released {
            let mouse_cursor = load::<Resource>("res://menu/sprite/mouse_cursor.png");
            Input::singleton().set_custom_mouse_cursor(&mouse_cursor);
        }
        if is_capture_pressed {
            self.capture_screen(false);
        }
    }

    fn process(&mut self, delta: f64) {
        let mut console = self.base().get_node_as::<TextEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/PanelContainer/VBoxContainer/console_text");
        match self.system_state {
            SystemState::IDLE => {
                self.time_accumulator = 0.0;
            },
            SystemState::CAPTURING => {
                // ---- CONSOLE UPDATES ----
                self.clear_errors();
                self.time_accumulator += delta as f32;
                if self.time_accumulator <= 0.1 {
                    console.set_text("Capturing Screen.");
                } else if self.time_accumulator > 0.1 && self.time_accumulator <= 0.2 {
                    console.set_text("Capturing Screen..");
                } else if self.time_accumulator > 0.2 && self.time_accumulator <= 0.3 {
                    console.set_text("Capturing Screen...");
                } else if self.time_accumulator > 0.3 && self.time_accumulator <= 0.4 {
                    self.time_accumulator = 0.0;
                }
                // ---- CATCH SCREEN CAPTURE ----
                let mut screen_queue = self.screen_queue.lock().unwrap();
                if let Some(screen_capture) = screen_queue.pop() {
                    let png_buffer = screen_capture.png_buffer;
                    let mut screen_image = Image::new_gd();
                    screen_image.load_png_from_buffer(&PackedByteArray::from(png_buffer.clone().into_inner().as_slice()));
                    let screen_texture = ImageTexture::create_from_image(&screen_image).expect("Failed to create ImageTexture!");
                    let mut screen_textrect = self.base().get_node_as::<TextureRect>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/PanelContainer/VBoxContainer/screen_textrect");
                    screen_textrect.set_texture(&screen_texture);
                    if screen_capture.is_preview == true {
                        console.set_text("Preview refreshed!");
                        self.system_state = SystemState::IDLE;
                    } else {
                        drop(screen_queue);
                        self.read_screen(png_buffer);
                    }
                }
                // ---- CATCH ERRORS ----
                let mut error_queue = self.error_queue.lock().unwrap();
                if let Some(eow) = error_queue.pop() {
                    let string = eow.string;
                    if eow.is_warning == true {
                        self.log_warning(string);
                    } else {
                        self.log_error(string);
                    }
                    self.system_state = SystemState::IDLE;
                }
            },
            SystemState::READING => {
                // ---- CONSOLE UPDATES ----
                self.clear_errors();
                self.time_accumulator += delta as f32;
                if self.time_accumulator <= 0.1 {
                    console.set_text("Capturing Screen Done!\nReading Screen.");
                } else if self.time_accumulator > 0.1 && self.time_accumulator <= 0.2 {
                    console.set_text("Capturing Screen Done!\nReading Screen..");
                } else if self.time_accumulator > 0.2 && self.time_accumulator <= 0.3 {
                    console.set_text("Capturing Screen Done!\nReading Screen...");
                } else if self.time_accumulator > 0.3 && self.time_accumulator <= 0.4 {
                    self.time_accumulator = 0.0;
                }
                // ---- BOX CONVEYOR ----
                let mut box_queue = self.box_queue.lock().unwrap();
                if let Some(()) = box_queue.pop() {
                    self.add_box_to_conveyor();
                }
                // ---- CATCH PACKETS ----
                let mut gui = self.base().get_node_as::<sandGUI>("sandGUI");
                let mut packets_queue = self.packets_queue.lock().unwrap();
                if let Some(packets) = packets_queue.pop() {
                    self.make_packets(gui, packets);
                    console.set_text("Capturing Screen Done!\nReading Screen Done!");
                    self.open_conveyor_boxes();
                    self.system_state = SystemState::IDLE;
                }
                // ---- CATCH ERRORS ----
                let mut error_queue = self.error_queue.lock().unwrap();
                if let Some(eow) = error_queue.pop() {
                    let string = eow.string;
                    if eow.is_warning == true {
                        self.log_warning(string);
                    } else {
                        self.log_error(string);
                    }
                    self.system_state = SystemState::IDLE;
                }
            },
            _ => {}
        }
    }
}

// UTILITY FUNCTIONS

fn create_vision_api_request(base64_image: String) -> Value {
    json!({
        "requests": [
            {
                "image": {
                    "content": base64_image
                },
                "features": [
                    {
                        "type": "DOCUMENT_TEXT_DETECTION"
                    }
                ]
            }
        ]
    })
}

async fn send_vision_api_request(request_body: Value, access_token: &str, project_id: &str) -> Result<Value, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client.post("https://vision.googleapis.com/v1/images:annotate")
        .bearer_auth(access_token)
        .header("x-goog-user-project", project_id)
        .json(&request_body)
        .send()
        .await?
        .json::<Value>()
        .await?;

    Ok(response)
}

async fn send_deepl_api_request(text: &str, auth_key: &str) -> Result<Value, reqwest::Error> {
    let client = reqwest::Client::new();
    let params = [("text", text), ("target_lang", "EN")];
    let response = client.post("https://api-free.deepl.com/v2/translate")
        .header("Authorization", format!("DeepL-Auth-Key {}", auth_key))
        .form(&params)
        .send()
        .await?
        .json::<Value>()
        .await?;

    Ok(response)
}

async fn parse_vision_response(response_json: Value, deepl_token: &str, box_queue_clone: Arc<Mutex<Vec<()>>>) -> Result<Vec<TranslationPacket>, ErrorOrWarning> {
    let mut packets = Vec::new();
    if let Some(pages) = response_json["responses"][0]["fullTextAnnotation"]["pages"].as_array() {
        for page in pages {
            if let Some(blocks) = page["blocks"].as_array() {
                for block in blocks {
                    if let Some(paragraphs) = block["paragraphs"].as_array() {
                        for paragraph in paragraphs {
                            let mut block_text = String::new();
                            for word in paragraph["words"].as_array().unwrap() {
                                for symbol in word["symbols"].as_array().unwrap() {
                                    block_text.push_str(symbol["text"].as_str().unwrap());
                                    if let Some(detected_break) = symbol["property"]["detectedBreak"].as_object() {
                                        if detected_break.contains_key("type") {
                                            block_text.push(' ');
                                        }
                                    }
                                }
                            }
                            // ---- TRANSLATION PACKET PREP ----
                            if kakasi::is_japanese(&block_text) == IsJapanese::False {
                                continue;
                            }
                            // KAKASI
                            let romaji_text = kakasi::convert(&block_text).romaji.into();
                            // DEEPL TRANSLATION
                            match send_deepl_api_request(&block_text, deepl_token).await {
                                Ok(response) => {
                                    godot_print!("DeepL response received!");
                                    let translation_text = parse_deepl_response(response);
                                    // WRAPUP
                                    packets.push(TranslationPacket {
                                        jp_text: block_text,
                                        jp_read: romaji_text,
                                        eng_text: translation_text,
                                    });
                                    let mut box_queue = box_queue_clone.lock().unwrap();
                                    box_queue.push(());
                                }
                                Err(error) => {
                                    godot_print!("DeepL error!");
                                    let eow = ErrorOrWarning {
                                        string: (format!("Failed to communicate with DeepL, check credentials!\n{}", error)),
                                        is_warning: false,
                                    };
                                    return Err(eow);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    Ok(packets)
}

fn parse_deepl_response(response_json: Value) -> String {
    if let Some(translation) = response_json["translations"][0]["text"].as_str() {
        return translation.to_string();
    } else {
        return "Translation failed!".to_string();
    }
}

#[godot_api]
impl System {
    #[func]
    fn list_windows(&self) {
        let windows = Window::all().unwrap();
        let vec_string: Vec<String> = windows.into_iter()
            .filter(|w| !w.is_minimized())
            .map(|w| format!("{}", w.title()))
            .collect();
        let mut window_selector = self.base().get_node_as::<OptionButton>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer10/OptionButton");
        window_selector.clear();
        window_selector.add_item("None");
        window_selector.select(0);
        for string in vec_string.iter() {
            godot_print!("WINDOW FOUND: {:?}", string);
            window_selector.add_item(string);
        }
    }

    fn add_box_to_conveyor(&self) {
        let mut box_conveyor = self.base().get_node_as::<HBoxContainer>("sandGUI/MarginContainer/VBoxContainer/vbox_content/PanelContainer/VBoxContainer/HBoxContainer");
        if box_conveyor.get_child_count() == 0 {
            let translating_text = load::<PackedScene>("res://translating_text.tscn").instantiate_as::<RichTextLabel>();
            make_child(&mut box_conveyor, translating_text.upcast());
        }
        let box_icon = load::<PackedScene>("res://box.tscn").instantiate_as::<TextureRect>();
        make_child(&mut box_conveyor, box_icon.upcast());
    }

    fn open_conveyor_boxes(&self) {
        godot_print!("OPENING CONVEYOR BOXES!");
        let mut box_conveyor = self.base().get_node_as::<HBoxContainer>("sandGUI/MarginContainer/VBoxContainer/vbox_content/PanelContainer/VBoxContainer/HBoxContainer");
        let box_count = box_conveyor.get_child_count();
        godot_print!("Box Count: {}", box_count);
        reset(box_conveyor.clone().upcast());
        let translating_text = load::<PackedScene>("res://translating_text.tscn").instantiate_as::<RichTextLabel>();
        make_child(&mut box_conveyor, translating_text.upcast());
        for i in 0..(box_count - 1) {
            let box_icon_open = load::<PackedScene>("res://box_open.tscn").instantiate_as::<TextureRect>();
            make_child(&mut box_conveyor, box_icon_open.clone().upcast());
        }
    }

    fn reset_box_conveyor(&self) {
        let mut box_conveyor = self.base().get_node_as::<HBoxContainer>("sandGUI/MarginContainer/VBoxContainer/vbox_content/PanelContainer/VBoxContainer/HBoxContainer");
        reset(box_conveyor.clone().upcast());
    }

    fn read_screen(&mut self, png_buffer: Cursor<Vec<u8>>) {
        self.system_state = SystemState::READING;
        let base64_encoded_image = encode(&png_buffer.into_inner());
        let request_body = create_vision_api_request(base64_encoded_image);
        // CREDENTIALS
        let gcloud_token = self.base().get_node_as::<TextEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/TextEdit").get_text().to_string();
        let project_id = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer8/LineEdit").get_text().to_string();
        let deepl_token = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer9/LineEdit").get_text().to_string();

        let packets_queue_clone = Arc::clone(&self.packets_queue);
        let error_queue_clone = Arc::clone(&self.error_queue);
        let box_queue_clone = Arc::clone(&self.box_queue);

        thread::spawn(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                match send_vision_api_request(request_body, &gcloud_token, &project_id).await {
                    Ok(response) => {
                        godot_print!("Google Cloud Vision response received!");
                        let result = parse_vision_response(response.clone(), &deepl_token, box_queue_clone).await;
                        match result {
                            Ok(packets) => {
                                if packets.is_empty() {
                                    let mut error_queue = error_queue_clone.lock().unwrap();
                                    let eow = ErrorOrWarning {
                                        string: (format!("Empty reading! It may be that there is no text in the reading area. Otherwise, you may need to update your credentials. Response JSON: {}", response)),
                                        is_warning: true,
                                    };
                                    error_queue.push(eow);
                                } else {
                                    let mut packets_queue = packets_queue_clone.lock().unwrap();
                                    packets_queue.push(packets);
                                }
                            },
                            Err(eow) => {
                                let mut error_queue = error_queue_clone.lock().unwrap();
                                error_queue.push(eow);
                            }
                        }
                    }
                    Err(error) => {
                        godot_print!("Google Cloud Vision error!");
                        let mut error_queue = error_queue_clone.lock().unwrap();
                        let eow = ErrorOrWarning {
                            string: (format!("Failed to communicate Google Cloud Vision: {}", error)),
                            is_warning: false,
                        };
                        error_queue.push(eow);
                    }
                }
            });
        });
    }

    #[func]
    fn capture_screen(&mut self, is_preview: bool) {
        self.reset_box_conveyor();
        let up_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit").get_text();
        let down_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit2").get_text();
        let left_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit3").get_text();
        let right_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit4").get_text();
        let margin_up = up_text.to_string().parse::<u32>().unwrap();
        let margin_down = down_text.to_string().parse::<u32>().unwrap();
        let margin_left = left_text.to_string().parse::<u32>().unwrap();
        let margin_right = right_text.to_string().parse::<u32>().unwrap();

        self.system_state = SystemState::CAPTURING;
        let mut png_buffer = Cursor::new(Vec::new());
        let screen_queue_clone = Arc::clone(&self.screen_queue);
        let error_queue_clone = Arc::clone(&self.error_queue);
        let window_selector = self.base().get_node_as::<OptionButton>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer10/OptionButton");
        let window_title = window_selector.get_text().to_string();
        if window_title == "None" {
            let mut error_queue = error_queue_clone.lock().unwrap();
            error_queue.push(ErrorOrWarning {
                string: format!("Please select a target window!"),
                is_warning: true,
            });
            return;
        }
        thread::spawn(move || {
            let windows = Window::all().unwrap();
            let window = windows.into_iter().find(|w| w.title() == window_title && !w.is_minimized());

            if let Some(window) = window {
                let image = match window.capture_image() {
                    Ok(img) => img,
                    Err(e) => {
                        let mut error_queue = error_queue_clone.lock().unwrap();
                        error_queue.push(ErrorOrWarning {
                            string: format!("Window capturing failure: {}", e),
                            is_warning: false,
                        });
                        return;
                    }
                };

                let new_width = image.width().saturating_sub(margin_left + margin_right);
                let new_height = image.height().saturating_sub(margin_up + margin_down);
                let cropped_image = image.view(margin_left, margin_up, new_width, new_height).to_image();
                let dynamic_img = DynamicImage::ImageRgba8(cropped_image);
                dynamic_img.write_to(&mut png_buffer, ImageOutputFormat::Png).unwrap();

                if !png_buffer.get_ref().is_empty() {
                    let mut screen_queue = screen_queue_clone.lock().unwrap();
                    let screen_capture = ScreenCapture { png_buffer, is_preview };
                    screen_queue.push(screen_capture);
                }
            } else {
                let mut error_queue = error_queue_clone.lock().unwrap();
                error_queue.push(ErrorOrWarning {
                    string: format!("Target window not found: make sure to refresh window list!"),
                    is_warning: false,
                });
                return;
            }
        });
    }

    #[func]
    fn save_credentials(&self) {
        // CREDENTIALS
        let gcloud_token = self.base().get_node_as::<TextEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/TextEdit").get_text().to_string();
        let project_id = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer8/LineEdit").get_text().to_string();
        let deepl_token = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer9/LineEdit").get_text().to_string();

        let user_credentials = Some(UserCredentials {
            gcloud_token,
            project_id,
            deepl_token,
        });

        let mut user_settings = UserSettings {
            user_credentials,
            reading_area: None,
            packet_config: None,
        };

        // PULL
        if FileAccess::file_exists("user://user_settings.toml") {
            let mut file = FileAccess::open("user://user_settings.toml", ModeFlags::READ).expect("Failed to open file!");
            let contents = file.get_as_text().to_string();
            file.close();
            match toml::from_str::<UserSettings>(&contents) {
                Ok(pulled_user_settings) => {
                    if let Some(reading_area) = pulled_user_settings.reading_area {
                        user_settings.reading_area =  Some(reading_area);
                    }
                    if let Some(packet_config) = pulled_user_settings.packet_config {
                        user_settings.packet_config =  Some(packet_config);
                    }
                },
                _ => {},
            }
        }

        // PUSH
        if let Ok(serialized) = toml::to_string(&user_settings) {
            let mut file = FileAccess::open("user://user_settings.toml", ModeFlags::WRITE).expect("Internal Error: Failed to open file!");
            file.store_string(&serialized);
            file.close();
            let mut console = self.base().get_node_as::<TextEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/PanelContainer/VBoxContainer/console_text");
            self.clear_errors();
            self.reset_box_conveyor();
            console.set_text("Credentials saved!");
        } else { self.log_error("Failed to save Credentials! You may have used invalid values.".to_string()); }
    }

    #[func]
    fn save_reading_area(&self) {
        // READING AREA
        let up_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit").get_text();
        let down_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit2").get_text();
        let left_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit3").get_text();
        let right_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit4").get_text();
        let up = up_text.to_string().parse::<u32>().unwrap();
        let down = down_text.to_string().parse::<u32>().unwrap();
        let left = left_text.to_string().parse::<u32>().unwrap();
        let right = right_text.to_string().parse::<u32>().unwrap();

        let reading_area = Some(ReadingArea {
            up,
            down,
            left,
            right,
        });


        let mut user_settings = UserSettings {
            user_credentials: None,
            reading_area,
            packet_config: None,
        };

        // PULL
        if FileAccess::file_exists("user://user_settings.toml") {
            let mut file = FileAccess::open("user://user_settings.toml", ModeFlags::READ).expect("Failed to open file!");
            let contents = file.get_as_text().to_string();
            file.close();
            match toml::from_str::<UserSettings>(&contents) {
                Ok(pulled_user_settings) => {
                    if let Some(user_credentials) = pulled_user_settings.user_credentials {
                        user_settings.user_credentials =  Some(user_credentials);
                    }
                    if let Some(packet_config) = pulled_user_settings.packet_config {
                        user_settings.packet_config =  Some(packet_config);
                    }
                },
                _ => {},
            }
        }

        // PUSH
        if let Ok(serialized) = toml::to_string(&user_settings) {
            let mut file = FileAccess::open("user://user_settings.toml", ModeFlags::WRITE).expect("Internal Error: Failed to open file!");
            file.store_string(&serialized);
            file.close();
            let mut console = self.base().get_node_as::<TextEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/PanelContainer/VBoxContainer/console_text");
            self.clear_errors();
            self.reset_box_conveyor();
            console.set_text("Crop Values saved!");
        } else { self.log_error("Failed to save Crop Values! You may have used invalid values.".to_string()); }
    }

    #[func]
    fn save_packet_config(&self) {
        // TRANSLATION PACKET CONFIG
        let jp_font = self.base().get_node_as::<OptionButton>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer4/OptionButton").get_selected_id();
        let font_size_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer/LineEdit").get_text();
        let font_size = font_size_text.to_string().parse::<usize>().unwrap();
        let romaji = self.base().get_node_as::<CheckButton>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer2/CheckButton").is_pressed();

        let packet_config = Some(PacketConfig {
            jp_font,
            font_size,
            romaji,
        });


        let mut user_settings = UserSettings {
            user_credentials: None,
            reading_area: None,
            packet_config,
        };

        // PULL
        if FileAccess::file_exists("user://user_settings.toml") {
            let mut file = FileAccess::open("user://user_settings.toml", ModeFlags::READ).expect("Failed to open file!");
            let contents = file.get_as_text().to_string();
            file.close();
            match toml::from_str::<UserSettings>(&contents) {
                Ok(pulled_user_settings) => {
                    if let Some(user_credentials) = pulled_user_settings.user_credentials {
                        user_settings.user_credentials =  Some(user_credentials);
                    }
                    if let Some(reading_area) = pulled_user_settings.reading_area {
                        user_settings.reading_area =  Some(reading_area);
                    }
                },
                _ => {},
            }
        }

        // PUSH
        if let Ok(serialized) = toml::to_string(&user_settings) {
            let mut file = FileAccess::open("user://user_settings.toml", ModeFlags::WRITE).expect("Internal Error: Failed to open file!");
            file.store_string(&serialized);
            file.close();
            let mut console = self.base().get_node_as::<TextEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/PanelContainer/VBoxContainer/console_text");
            self.clear_errors();
            self.reset_box_conveyor();
            console.set_text("Translation Packet Config saved!");
        } else { self.log_error("Failed to save Packet Config Settings! You may have used invalid values.".to_string()); }
    }

    fn load_user_settings(&self) {
        let mut file = FileAccess::open("user://user_settings.toml", ModeFlags::READ).expect("Failed to open file!");
        let contents = file.get_as_text().to_string();
        file.close();
        match toml::from_str::<UserSettings>(&contents) {
            Ok(user_settings) => {
                // SET CREDENTIALS
                if let Some(user_credentials) = user_settings.user_credentials {
                    let mut gcloud_token = self.base().get_node_as::<TextEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/TextEdit");
                    let mut project_id = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer8/LineEdit");
                    let mut deepl_token = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer9/LineEdit");
                    gcloud_token.set_text(&user_credentials.gcloud_token);
                    project_id.set_text(&user_credentials.project_id);
                    deepl_token.set_text(&user_credentials.deepl_token);
                }
                // SET READING AREA
                if let Some(reading_area) = user_settings.reading_area {
                    let mut up = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit");
                    let mut down = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit2");
                    let mut left = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit3");
                    let mut right = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit4");
                    up.set_text(&reading_area.up.to_string());
                    down.set_text(&reading_area.down.to_string());
                    left.set_text(&reading_area.left.to_string());
                    right.set_text(&reading_area.right.to_string());
                }
                // SET PACKET CONFIG
                if let Some(packet_config) = user_settings.packet_config {
                    let mut jp_font = self.base().get_node_as::<OptionButton>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer4/OptionButton");
                    let mut font_size = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer/LineEdit");
                    let mut romaji = self.base().get_node_as::<CheckButton>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer2/CheckButton");
                    jp_font.select(packet_config.jp_font);
                    font_size.set_text(&packet_config.font_size.to_string());
                    romaji.set_pressed(packet_config.romaji);
                }
            },
            Err(error) => {
                self.log_error(format!("Failed to load user settings: {}", error));
            }
        }
    }

    fn log_error(&self, error_string: String) {
        self.reset_box_conveyor();
        let mut vbox_error = self.base().get_node_as::<VBoxContainer>("sandGUI/MarginContainer/VBoxContainer/vbox_content/PanelContainer/vbox_error");
        let mut console_error = vbox_error.get_node_as::<TextEdit>("console_error");
        console_error.set_text(&error_string);
        vbox_error.set_visible(true);
    }

    fn log_warning(&self, warning_string: String) {
        self.reset_box_conveyor();
        let mut vbox_warning = self.base().get_node_as::<VBoxContainer>("sandGUI/MarginContainer/VBoxContainer/vbox_content/PanelContainer/vbox_warning");
        let mut console_warning = vbox_warning.get_node_as::<TextEdit>("console_warning");
        console_warning.set_text(&warning_string);
        vbox_warning.set_visible(true);
    }

    fn clear_errors(&self) {
        let mut vbox_error = self.base().get_node_as::<VBoxContainer>("sandGUI/MarginContainer/VBoxContainer/vbox_content/PanelContainer/vbox_error");
        let mut console_error = vbox_error.get_node_as::<TextEdit>("console_error");
        let mut vbox_warning = self.base().get_node_as::<VBoxContainer>("sandGUI/MarginContainer/VBoxContainer/vbox_content/PanelContainer/vbox_warning");
        let mut console_warning = vbox_warning.get_node_as::<TextEdit>("console_warning");
        vbox_error.set_visible(false);
        console_error.clear();
        vbox_warning.set_visible(false);
        console_warning.clear();
    }

    fn make_packets(&self, mut gui: Gd<sandGUI>, packets: Vec<TranslationPacket>) {
        let mut vbox = gui.get_node_as::<VBoxContainer>("MarginContainer/VBoxContainer/vbox_content/TabContainer/Reader/PanelContainer/ScrollContainer/VBoxContainer");
        reset(vbox.clone().upcast());
        godot_print!("Packets found: {}", packets.len());
        for packet in packets {
            let mut translation_packet = load::<PackedScene>("res://translation_packet.tscn").instantiate_as::<PanelContainer>();
            let mut jp_text = translation_packet.get_node_as::<RichTextLabel>("VBoxContainer/jptext_container/jptext");
            let mut jp_read = translation_packet.get_node_as::<RichTextLabel>("VBoxContainer/jpread_container/jpread");
            let mut eng_text = translation_packet.get_node_as::<RichTextLabel>("VBoxContainer/engtext_container/engtext");
            jp_text.set_text(&packet.jp_text.clone());
            jp_read.set_text(&packet.jp_read);
            eng_text.set_text(&packet.eng_text);
            self.post_process_packet(&mut translation_packet);
            make_child(&mut vbox, translation_packet.clone().upcast());
            gui.bind_mut().fade_in(translation_packet.upcast());
        }
    }

    fn post_process_packet(&self, translation_packet: &mut Gd<PanelContainer>) {
        // --- APPLY USER SETTINGS ---
        let gui = self.base().get_node_as::<sandGUI>("sandGUI");
        let mut jp_text = translation_packet.get_node_as::<RichTextLabel>("VBoxContainer/jptext_container/jptext");
        let mut jp_read = translation_packet.get_node_as::<RichTextLabel>("VBoxContainer/jpread_container/jpread");
        let mut eng_text = translation_packet.get_node_as::<RichTextLabel>("VBoxContainer/engtext_container/engtext");
        // -- JP FONT --
        let jp_font = gui.get_node_as::<OptionButton>("MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer4/OptionButton").get_selected_id();
        if jp_font == 0 {
            let font_dotgothic = load::<Font>("res://menu/font/DotGothic16-Regular.ttf");
            jp_text.add_theme_font_override("normal_font", &font_dotgothic);
        } else if jp_font == 1 {
            let font_shippori = load::<Font>("res://menu/font/ShipporiMincho-Regular.ttf");
            jp_text.add_theme_font_override("normal_font", &font_shippori);
        }
        // -- FONT SIZE --
        let mut font_size_box = gui.get_node_as::<LineEdit>("MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer/LineEdit");
        let font_size = font_size_box.get_text().to_string().parse::<i32>().unwrap().max(12).min(36);
        font_size_box.set_text(&font_size.to_string());
        jp_text.add_theme_font_size_override("normal_font_size", font_size);
        jp_read.add_theme_font_size_override("normal_font_size", font_size);
        eng_text.add_theme_font_size_override("normal_font_size", font_size);
        // -- ROMAJI --
        let romaji = gui.get_node_as::<CheckButton>("MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer2/CheckButton").is_pressed();
        if romaji == true {
            jp_read.set_visible(true);
        } else {
            jp_read.set_visible(false);
        }
    }

    #[func]
    fn refresh_preview_packet(&self) {
        let mut translation_packet = self.base().get_node_as::<PanelContainer>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/translation_packet");
        self.post_process_packet(&mut translation_packet);
    }
}
