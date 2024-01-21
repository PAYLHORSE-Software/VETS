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
