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

use std::process::Command;
use std::io::{Write, Read};
use std::env;
use std::path::Path;

fn usage(program: &str) {
    eprintln!("Usage: '{}' <engine-1> [engine-2]", program);
}

fn main() {
    let mut args = env::args();
    let program = args.next().expect("Expected program name");

    if !Path::new("engines/").exists() {
        eprintln!("There needs to be an engines/ directory");
        std::process::exit(1);
    }

    if args.len() < 1 {
        usage(&program);
        std::process::exit(0);
    }

    let engine_a = args.next();
    let engine_b = args.next();

    if let Some(ref a) = engine_a {
        let engine_path = &format!("engines/{}", a);
        if !Path::new(engine_path).exists() {
            eprintln!("'{}' needs to be located in the engines/ directory", a);
            std::process::exit(1);
        }
    }

    if let Some(ref b) = engine_b {
        let engine_path = &format!("engines/{}", b);
        if !Path::new(engine_path).exists() {
            eprintln!("'{}' needs to be located in the engines/ directory", b);
            std::process::exit(1);
         }
    }

    if let Err(e) = gui_main::gui_main(engine_a, engine_b) {
        eprintln!("[ERROR] Something went wrong!");
        eprintln!("[ERROR] {e}");
    }
}
