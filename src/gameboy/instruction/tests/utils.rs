use crate::gameboy::{Gameboy};

pub fn load_instr(gb: &mut Gameboy, bytes: Vec<u8>) {
    let mut rom = Box::new([0; 0x8000]);
    for (i, byte) in bytes.iter().enumerate() {
        rom[(gb.pc as usize) + i] = *byte;
    }
    gb.load_rom(&rom);
}
