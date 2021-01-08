mod gameboy;

use crate::gameboy::{Gameboy, step};

fn main() {
    let mut gb = Gameboy::new();

    // LD B, 0xEE
    gb.mem[0x0100] = 0x06;
    gb.mem[0x0101] = 0xee;
    // LD C, B
    gb.mem[0x0102] = 0x48;
    // NOP
    gb.mem[0x0103] = 0x00;
    // PUSH BC
    gb.mem[0x0104] = 0xc5;
    // POP DE
    gb.mem[0x0105] = 0xd1;
    
    println!("{}\n", gb);

    step_display(&mut gb, "LD B, 0xEE");
    step_display(&mut gb, "LD C, B");
    step_display(&mut gb, "NOP");
    step_display(&mut gb, "PUSH BC");
    step_display(&mut gb, "POP DE");
}

fn step_display(gb: &mut Gameboy, instr_mnemonic: &str) {
    println!("{}", instr_mnemonic);
    step(gb);
    println!("{}\n", gb);
}