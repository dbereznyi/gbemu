mod gameboy;

use crate::gameboy::{Gameboy, step};

fn main() {
    let mut gb = Gameboy::new();
    run_test_program(&mut gb);
}

fn run_test_program(gb: &mut Gameboy) {
    let program = [
        ("LD B, 0xEE", vec!(0x06, 0xee)),
        ("LD C, B", vec!(0x48)),
        ("NOP", vec!(0x00)),
        ("PUSH BC", vec!(0xc5)),
        ("POP DE", vec!(0xd1)),
        ("ADD HL, DE", vec!(0x19)),
        ("LD A, 0x01", vec!(0x3e, 0x01)),
        ("SUB 0x01", vec!(0xd6, 0x01)),
    ];

    // Load program
    let mut i = 0x0100;
    for (_, bytes) in program.iter() {
        for byte in bytes.iter() {
            gb.mem[i] = *byte;
            i += 1;
        }
    }
    
    // Execute program
    println!("==> initial state\n{}\n", gb);
    for (mnemonic, _) in program.iter() {
        step_display(gb, mnemonic);
    }
}

fn step_display(gb: &mut Gameboy, mnemonic: &str) {
    println!("==> {}", mnemonic);
    step(gb);
    println!("{}\n", gb);
}