use crate::gameboy::{Gameboy};

pub fn load_instr(gb: &mut Gameboy, bytes: Vec<u8>) {
    for (i, byte) in bytes.iter().enumerate() {
        gb.mem[(gb.pc + (i as u16)) as usize] = *byte;
    }
}