mod attack;
mod bb;
mod board;
mod consts;
mod fen;
mod gui_main;
mod magic_consts;
mod magics;
mod move_gen;
mod moves;

use std::{process::Command, io::{Write, Read}};

fn main() {
    if let Err(e) = gui_main::gui_main() {
        eprintln!("[ERROR] Something went wrong!");
        eprintln!("[ERROR] {e}");
    }
}
