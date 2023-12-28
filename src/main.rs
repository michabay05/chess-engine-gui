mod gui;
mod utils;
mod comm;

use std::env;

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

    if let Err(e) = gui::gui_main(engine_a.unwrap(), engine_b) {
        eprintln!("[ERROR] Something went wrong!");
        eprintln!("[ERROR] {e}");
    }
}
