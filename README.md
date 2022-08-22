# gbemu
A work-in-progress gameboy emulator written in Rust.

## Currently implemented
- All CPU instructions
- All PPU functions
- VBlank, LCDC, HI_LO, Timer interrupts
- ROM loading with support for different memory bank controllers (testing still needed)
- A primitive interactive text debugger

## Planned
- Serial functionality
- VRAM viewer
- More advance text debugger
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
