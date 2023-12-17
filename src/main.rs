mod attack;
mod bb;
mod board;
mod comm;
mod consts;
mod fen;
mod gui_main;
mod magic_consts;
mod magics;
mod move_gen;
mod moves;
mod zobrist;

use std::process::Command;
use std::io::{Write, Read};
use std::env;
use std::path::Path;

fn main() {
    let mut args = env::args();
    let program = args.next().expect("Expected program name");

    if args.len() < 1 {
        // the first engine is a requirement, the second one is optional
        eprintln!("Usage: '{}' <engine-1> [engine-2]", program);
        std::process::exit(1);
    }

    let engine_a = args.next();
    let engine_b = args.next();

    if let Err(e) = gui_main::gui_main(engine_a.unwrap(), engine_b) {
        eprintln!("[ERROR] Something went wrong!");
        eprintln!("[ERROR] {e}");
    }
}
