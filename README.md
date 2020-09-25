# Rust-Chip8-Emulator
A CHIP-8 Emulator written in rust, with piston

##Compilation
To compile, simply run cargo from the root main directory:
```bash
cargo build
```

##Usage
Run the emulator from the directory containing the assets folder
```bash
./chip8 "Path/To/Program.ch8"
```

##Controls
Keyboard inputs on the left correspond to the CHIP-8 keypad on the right
```
|-------|   |-------|
|1|2|3|4|   |1|2|3|C|
|-------|   |-------|
|Q|W|E|R|   |4|5|6|D|
|-------| = |-------|
|A|S|D|F|   |7|8|9|E|
|-------|   |-------|
|Z|X|C|V|   |A|0|B|F|
|-------|   |-------|
```
