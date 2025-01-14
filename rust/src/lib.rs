// ------ GODOT ENTRY POINT ------
#![allow(non_camel_case_types)]
#![allow(unused)]
fn main() {
use godot::prelude::*;

struct Rust;

#[gdextension]
unsafe impl ExtensionLibrary for Rust {}
}

// ------ MODULE IMPORT ------
mod system;
mod gui;

// ------ UTILITY FUNCTIONS ------
pub mod utils {
    use godot::prelude::*;
    // ________________________________________
    // Instantiate a Node as child of another
    // ----------------------------------------
    pub fn make_child<T: GodotClass + Inherits<Node>>(parent: &mut Gd<T>, child: Gd<Node>) {
        parent.clone().upcast::<Node>().add_child(&child);
    }
    // __________________________________________
    // Reset a root node: delete all child nodes
    // ------------------------------------------
    pub fn reset(node: Gd<Node>) {
        let children = node.get_children();
        for mut child in children.iter_shared() {
            // godot_print!("REMOVED: {}", child);
            child.queue_free();
        }
    }
}
