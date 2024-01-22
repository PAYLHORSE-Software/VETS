use godot::prelude::*;
use godot::engine::{InputEvent, Control, PanelContainer, VBoxContainer, Image, ImageTexture, TextureRect};
use crate::utils::*;
use crate::gui::sandGUI;
use scrap::{Capturer, Display};
use std::io::ErrorKind::WouldBlock;
use std::thread;
use std::time::Duration;
use image::{ImageBuffer, Rgba, ImageOutputFormat};
use std::io::Cursor;

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
            // -- DEBUG: Spawn in Translation Packet --
            let mut gui = self.base().get_node_as::<sandGUI>("sandGUI");
            let mut vbox = gui.get_node_as::<VBoxContainer>("MarginContainer/VBoxContainer/vbox_content/TabContainer/Reader/VBoxContainer");
            let packet = load::<PackedScene>("res://translation_packet.tscn").instantiate_as::<PanelContainer>();
            make_child(&mut vbox, packet.clone().upcast());
            gui.bind_mut().fade_in(packet.upcast());
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
            self.window_capture();
        }
    }

    fn process(&mut self, delta: f64) {}
}

fn normalized(filename: &str) -> String {
    filename
        .replace("|", "")
        .replace("\\", "")
        .replace(":", "")
        .replace("/", "")
}

#[godot_api]
impl System {
    pub fn read_screen(&mut self) {
        godot_print!("Reading screen...");
    }

    fn window_capture(&self) {
        // TODO: READING AREA update
        println!("Capturing screen...");
        let display = Display::primary().expect("Couldn't find primary display.");
        let mut capturer = Capturer::new(display).expect("Couldn't begin capture.");
        let (w, h) = (capturer.width(), capturer.height());

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
            let img: ImageBuffer<Rgba<u8>, Vec<u8>> = ImageBuffer::from_raw(w as u32, h as u32, frame.to_vec()).unwrap();
            let mut png_buffer = Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(img).write_to(&mut png_buffer, ImageOutputFormat::Png).unwrap();
            png_buffer.set_position(0);
            println!("Captured a frame of size {}", frame.len());
            let mut screen_image = Image::new();
            screen_image.load_png_from_buffer(PackedByteArray::from(png_buffer.into_inner().as_slice()));
            let screen_texture = ImageTexture::create_from_image(screen_image).expect("Failed to create ImageTexture!");
            let mut screen_textrect = self.base().get_node_as::<TextureRect>("sandGUI/MarginContainer/VBoxContainer/vbox_content/TabContainer/Settings/ScrollContainer/VBoxContainer/PanelContainer/VBoxContainer/screen_textrect");
            screen_textrect.set_texture(screen_texture.upcast());
            break;
        }
    }
}
