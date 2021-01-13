mod gameboy;

use std::io::{self, Write};
use std::collections::HashMap;
use crate::gameboy::{Gameboy, step};

fn main() -> std::io::Result<()> {
    let mut gb = Gameboy::new();
    
    let program = vec!(
        // ("LD B, 0xEE", vec!(0x06, 0xee)),
        // ("LD C, B", vec!(0x48)),
        // ("NOP", vec!(0x00)),
        // ("PUSH BC", vec!(0xc5)),
        // ("POP DE", vec!(0xd1)),
        // ("ADD HL, DE", vec!(0x19)),
        // ("LD A, 0x01", vec!(0x3e, 0x01)),
        // ("SUB 0x01", vec!(0xd6, 0x01)),
        // ("JP 0x0101", vec!(0xc3, 0x01, 0x01)),
        ("LD A, 0xBB", vec!(0x3e, 0xbb)),
        ("RLCA", vec!(0x07)),
        ("LD A, 0xBB", vec!(0x3e, 0xbb)),
        ("RLA", vec!(0x17)),
        ("LD A, 0x77", vec!(0x3e, 0x77)),
        ("RLA", vec!(0x17)),
    );

    run_test_program(&mut gb, program);
    Ok(())
}

fn run_test_program(gb: &mut Gameboy, program: Vec<(&str, Vec<u8>)>) {
    let mut addr_to_mnemonic = HashMap::new();

    // Load program
    let mut addr = 0x0100;
    for (mnemonic, bytes) in program.iter() {
        addr_to_mnemonic.insert(addr, mnemonic);
        for byte in bytes.iter() {
            gb.mem[addr] = *byte;
            addr += 1;
        }
    }
    let program_end = addr as u16;

    println!("{:?}", addr_to_mnemonic);
    
    // Execute program
    println!("==> initial state\n{}\n", gb);
    let mut buf = String::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    while gb.pc < program_end {
        let mnemonic = addr_to_mnemonic.get(&(gb.pc as usize)).unwrap();
        print!("==> {}", mnemonic);
        stdout.flush().unwrap();
        stdin.read_line(&mut buf).unwrap();
        step(gb);
        println!("{}\n", gb);
    }
}