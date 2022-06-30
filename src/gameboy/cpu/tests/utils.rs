use crate::gameboy::cartridge::{Cartridge, load_cartridge};

pub fn test_cartridge(bytes: Vec<u8>) -> Cartridge {
    let mut rom = Box::new([0; 0x8000]);
    for (i, byte) in bytes.iter().enumerate() {
        rom[0x0100 + i] = *byte;
    }
    load_cartridge(&*rom).unwrap()
}
