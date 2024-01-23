use std::io::ErrorKind::WouldBlock;
use std::thread;
use std::time::Duration;
use std::io::Cursor;
use godot::prelude::*;
use godot::engine::{InputEvent, Control, PanelContainer, VBoxContainer, Image, ImageTexture, TextureRect, LineEdit, RichTextLabel};
use scrap::{Capturer, Display};
use image::{ImageBuffer, Rgba, ImageOutputFormat, GenericImageView};
use base64::encode;
use serde_json::{json, Value};
use reqwest;
use crate::utils::*;
use crate::gui::sandGUI;

struct TranslationPacket {
    jp_text: String,
}

#[derive(GodotClass)]
#[class(base = Node)]
pub struct System {
    #[base]
    pub node: Base<Node>,
}

#[godot_api]
impl INode for System {
    fn init(node: Base<Node>) -> Self {
        System {
            node,
        }
    }

    fn ready(&mut self) {
        godot_print!("Starting VETS...");
        // -- MOUSE CURSOR --
        let mouse_cursor = load::<Resource>("res://menu/sprite/mouse_cursor.png");
        Input::singleton().set_custom_mouse_cursor(mouse_cursor.upcast());
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        // --- MOUSE CURSOR ---
        let is_mouse_clicked = Input::singleton().is_action_just_pressed("mouse_click".into());
        let is_mouse_released = Input::singleton().is_action_just_released("mouse_click".into());
        let is_mouse_right_clicked = Input::singleton().is_action_just_pressed("mouse_rightclick".into());
        let is_mouse_right_released = Input::singleton().is_action_just_released("mouse_rightclick".into());
        let is_capture_pressed = Input::singleton().is_action_just_pressed("capture".into());
        if is_mouse_clicked {
            let mouse_cursor = load::<Resource>("res://menu/sprite/mouse_cursor_2.png");
            Input::singleton().set_custom_mouse_cursor(mouse_cursor.upcast());
        }
        if is_mouse_released {
            let mouse_cursor = load::<Resource>("res://menu/sprite/mouse_cursor.png");
            Input::singleton().set_custom_mouse_cursor(mouse_cursor.upcast());
        }
        if is_mouse_right_clicked {
            let mouse_cursor = load::<Resource>("res://menu/sprite/mouse_cursor_3.png");
            Input::singleton().set_custom_mouse_cursor(mouse_cursor.upcast());
        }
        if is_mouse_right_released {
            let mouse_cursor = load::<Resource>("res://menu/sprite/mouse_cursor.png");
            Input::singleton().set_custom_mouse_cursor(mouse_cursor.upcast());
        }
        if is_capture_pressed {
            self.read_screen();
        }
    }

    fn process(&mut self, delta: f64) {}
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

fn parse_vision_response(response_json: Value) -> Vec<TranslationPacket> {
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
                            packets.push(TranslationPacket { jp_text: block_text });
                        }
                    }
                }
            }
        }
    }
    packets
}

fn parse_deepl_response(response_json: Value) -> String {
    if let Some(translation) = response_json["translations"][0]["text"].as_str() {
        return translation.to_string();
    } else {
        return "Translation failed!".to_string();
    }
}

async fn new_translation(mut gui: Gd<sandGUI>, deepl_token: &str, packets: Vec<TranslationPacket>) {
    let mut vbox = gui.get_node_as::<VBoxContainer>("MarginContainer/VBoxContainer/vbox_content/TabContainer/Reader/VBoxContainer");
    reset(vbox.clone().upcast());
    godot_print!("Packets found: {}", packets.len());
    for packet in packets {
        let translation_packet = load::<PackedScene>("res://translation_packet.tscn").instantiate_as::<PanelContainer>();
        let mut jp_text = translation_packet.get_node_as::<RichTextLabel>("VBoxContainer/jptext_container/jptext");
        let mut jp_read = translation_packet.get_node_as::<RichTextLabel>("VBoxContainer/jpread_container/jpread");
        let mut eng_text = translation_packet.get_node_as::<RichTextLabel>("VBoxContainer/engtext_container/engtext");
        jp_text.set_text(packet.jp_text.clone().into());
        // KAKASI
        jp_read.set_text(kakasi::convert(&packet.jp_text).romaji.into());
        // DEEPL TRANSLATION
        match send_deepl_api_request(&packet.jp_text, deepl_token).await {
            Ok(response) => {
                godot_print!("DeepL response received!");
                let translation_text = parse_deepl_response(response);
                eng_text.set_text(translation_text.into());
                make_child(&mut vbox, translation_packet.clone().upcast());
                gui.bind_mut().fade_in(translation_packet.upcast());
            }
            Err(e) => {
                godot_print!("Error sending translation request: {:?}", e);
            }
        }
    }
}

#[godot_api]
impl System {
    pub fn read_screen(&mut self) {
        godot_print!("Reading screen...");
        let png_buffer = self.capture_screen_return();
        let base64_encoded_image = encode(&png_buffer.into_inner());
        let request_body = create_vision_api_request(base64_encoded_image);
        // CREDENTIALS
        let gcloud_token = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/TextEdit").get_text().to_string();
        let project_id = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer8/LineEdit").get_text().to_string();
        let deepl_token = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer9/LineEdit").get_text().to_string();

        tokio::runtime::Runtime::new().unwrap().block_on(async {
            match send_vision_api_request(request_body, &gcloud_token, &project_id).await {
                Ok(response) => {
                    godot_print!("Response received!");
                    // godot_print!("Response JSON: {}", response);
                    let packets = parse_vision_response(response);
                    let mut gui = self.base().get_node_as::<sandGUI>("sandGUI");
                    new_translation(gui, &deepl_token, packets).await;
                }
                Err(e) => {
                    godot_print!("Error sending request: {:?}", e);
                }
            }
        });
    }

    #[func]
    fn capture_screen(&self) {
        // User-defined dimensions and starting coordinates
        let start_x_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit").get_text();
        let start_y_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit2").get_text();
        let width_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer5/LineEdit").get_text();
        let height_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer6/LineEdit").get_text();
        let start_x = start_x_text.to_string().parse::<usize>().unwrap();
        let start_y = start_y_text.to_string().parse::<usize>().unwrap();
        let width = width_text.to_string().parse::<usize>().unwrap();
        let height = height_text.to_string().parse::<usize>().unwrap();

        println!("Capturing screen...");
        let display = Display::primary().expect("Couldn't find primary display.");
        let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");
        let (x, y) = (capturer.width(), capturer.height());

        let mut png_buffer = Cursor::new(Vec::new());

        loop {
            let frame = match capturer.frame() {
                Ok(frame) => frame,
                Err(error) => {
                    if error.kind() == WouldBlock {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    } else {
                        panic!("Error: {}", error);
                    }
                }
            };

            // Convert the frame (Vec<u8>) into an ImageBuffer
            let full_img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(x as u32, y as u32, frame.to_vec()).unwrap();

            // Crop the image to the desired dimensions and starting coordinates
            let cropped_img = full_img.view(start_x as u32, start_y as u32, width as u32, height as u32).to_image();

            // Encode the cropped ImageBuffer into a PNG buffer
            png_buffer = Cursor::new(Vec::new());
            let dynamic_img = image::DynamicImage::ImageRgba8(cropped_img);
            dynamic_img.write_to(&mut png_buffer, ImageOutputFormat::Png).unwrap();

            println!("Captured a frame of size {}", frame.len());
            let mut screen_image = Image::new();
            screen_image.load_png_from_buffer(PackedByteArray::from(png_buffer.clone().into_inner().as_slice()));
            let screen_texture = ImageTexture::create_from_image(screen_image).expect("Failed to create ImageTexture!");
            let mut screen_textrect = self.base().get_node_as::<TextureRect>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/PanelContainer/VBoxContainer/screen_textrect");
            screen_textrect.set_texture(screen_texture.upcast());
            break;
        }
    }

    fn capture_screen_return(&self) -> Cursor<Vec<u8>> {
        // User-defined dimensions and starting coordinates
        let start_x_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit").get_text();
        let start_y_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer7/LineEdit2").get_text();
        let width_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer5/LineEdit").get_text();
        let height_text = self.base().get_node_as::<LineEdit>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/HBoxContainer6/LineEdit").get_text();
        let start_x = start_x_text.to_string().parse::<usize>().unwrap();
        let start_y = start_y_text.to_string().parse::<usize>().unwrap();
        let width = width_text.to_string().parse::<usize>().unwrap();
        let height = height_text.to_string().parse::<usize>().unwrap();

        println!("Capturing screen...");
        let display = Display::primary().expect("Couldn't find primary display.");
        let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");
        let (x, y) = (capturer.width(), capturer.height());

        let mut png_buffer = Cursor::new(Vec::new());

        loop {
            let frame = match capturer.frame() {
                Ok(frame) => frame,
                Err(error) => {
                    if error.kind() == WouldBlock {
                        thread::sleep(Duration::from_millis(1));
                        continue;
                    } else {
                        panic!("Error: {}", error);
                    }
                }
            };

            // Convert the frame (Vec<u8>) into an ImageBuffer
            let full_img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(x as u32, y as u32, frame.to_vec()).unwrap();

            // Crop the image to the desired dimensions and starting coordinates
            let cropped_img = full_img.view(start_x as u32, start_y as u32, width as u32, height as u32).to_image();

            // Encode the cropped ImageBuffer into a PNG buffer
            png_buffer = Cursor::new(Vec::new());
            let dynamic_img = image::DynamicImage::ImageRgba8(cropped_img);
            dynamic_img.write_to(&mut png_buffer, ImageOutputFormat::Png).unwrap();

            println!("Captured a frame of size {}", frame.len());
            let mut screen_image = Image::new();
            screen_image.load_png_from_buffer(PackedByteArray::from(png_buffer.clone().into_inner().as_slice()));
            let screen_texture = ImageTexture::create_from_image(screen_image).expect("Failed to create ImageTexture!");
            let mut screen_textrect = self.base().get_node_as::<TextureRect>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/PanelContainer/VBoxContainer/screen_textrect");
            screen_textrect.set_texture(screen_texture.upcast());
            break;
        }
        return png_buffer;
    }

}
