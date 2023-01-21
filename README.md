# gbemu
A work-in-progress gameboy emulator written in Rust.

![Screenshot_20220822_173535](https://user-images.githubusercontent.com/45151389/186024984-d2ff380c-6e00-4011-9c27-ff2b991fd56b.png)
![Screenshot_20220822_173603](https://user-images.githubusercontent.com/45151389/186025011-ac10861d-4c49-4e7b-bd17-3c2f518cd599.png)
![Screenshot_20220822_174909](https://user-images.githubusercontent.com/45151389/186025090-d9f99fb0-fd3f-4bd7-a92d-cda0d87b44f9.png)

## Currently implemented
- All CPU instructions
- All PPU functions
- VBlank, LCDC, HI_LO, Timer interrupts
- ROM loading with support for different memory bank controllers (testing still needed)
- A primitive interactive text debugger

## Planned
- Serial functionality
- VRAM viewer
- More advanced text debugger
- Graphical debugger

## Building
```
cargo build
```

## Running
```
cargo run -r -- <path to a gameboy ROM file>
```
Additional options are available for changing the color palette, setting up breakpoints, and viewing CPU/PPU speed (see `main.rs`).

Test ROMS are available under the `roms/` directory.
