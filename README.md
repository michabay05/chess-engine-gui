# Chess Engine GUI

## Quickstart
The GUI requires all the engines provided in the command line argument to be located in an `engines` directory. 
Since the GUI can only communicate via the UCI protocol, the engines have to be able to use UCI for communications.
```
$ cargo build --release
$ ./engine-gui <engine-1> [engine-2]
```

## Resource
- [`haze-chess` repo](https://github.com/michabay05/haze-chess)
    - Borrowed board representation and move generation
- [`raylib-rs` repo](https://github.com/deltaphc/raylib-rs)
    - Used for window management and rendering
    - Rust binding for the [original C version of raylib](https://github.com/raysan5/raylib)
- [`chess.com-boards-and-pieces` repo](https://github.com/GiorgioMegrelli/chess.com-boards-and-pieces)
    - Got default [chess.com](https://www.chess.com/) pieces using this repo
