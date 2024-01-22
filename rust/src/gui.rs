use godot::prelude::*;
use godot::engine::{Control, IControl};

// ============================================================
// -- sandGUIManager --
// Manages visibility of GUI elements
// ============================================================
// TODO: Fix duplicates/panic
#[derive(GodotClass)]
#[class(base = Control)]
pub struct sandGUI {
    #[base]
    pub node: Base<Control>,
    pub queue_in: Vec<Gd<Control>>,
    pub queue_out: Vec<Gd<Control>>,
}

#[godot_api]
impl IControl for sandGUI {
    fn init(node: Base<Control>) -> Self {
        sandGUI {
            node,
            queue_in: Vec::new(),
            queue_out: Vec::new(),
        }
    }

    fn process(&mut self, delta: f64) {
        for i in 0..self.queue_in.len() {
            if i == 0 {
                let control = &mut self.queue_in[i];
                let new_alpha = (control.get_modulate().a + (4.0 * delta as f32)).min(1.0);
                // godot_print!("{} new alpha: {}", control.get_name(), new_alpha);
                control.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, new_alpha));
                if new_alpha >= 1.0 { self.queue_in.remove(i); }
            }
        }
        for i in 0..self.queue_out.len() {
            if i == 0 {
                let control = &mut self.queue_out[i];
                let new_alpha = (control.get_modulate().a - (4.0 * delta as f32)).max(0.0);
                // godot_print!("{} new alpha: {}", control.get_name(), new_alpha);
                control.set_modulate(Color::from_rgba(1.0, 1.0, 1.0, new_alpha));
                if new_alpha <= 0.0 {
                    control.set_visible(false);
                    self.queue_out.remove(i);
                }
            }
        }
    }
}

#[godot_api]
impl sandGUI {
    pub fn fade_in(&mut self, mut control: Gd<Control>) {
        godot_print!("FADING IN: {}", control.get_name());
        control.set_modulate(Color::TRANSPARENT_WHITE);
        control.set_visible(true);
        self.queue_in.push(control);
    }

    pub fn fade_out(&mut self, mut control: Gd<Control>) {
        godot_print!("FADING OUT: {}", control.get_name());
        self.queue_out.push(control);
    }
}
