use std::fmt;
use std::sync::atomic::{AtomicU8};
use std::convert::TryInto;

// Registers are referred to by indexing into Gameboy.regs
/// The type of an 8-bit register
pub type R = usize;
/// The type of a 16-bit register, i.e. a pair of 8-bit registers used together
pub type RR = (usize, usize);

// 8-bit register aliases
pub const RB: R = 0;
pub const RC: R = 1;
pub const RD: R = 2;
pub const RE: R = 3;
pub const RH: R = 4;
pub const RL: R = 5;
pub const RF: R = 6;
pub const RA: R = 7;

// 16-bit register aliases
pub const RAF: RR = (RA, RF);
pub const RBC: RR = (RB, RC);
pub const RDE: RR = (RD, RE);
pub const RHL: RR = (RH, RL);

// Flag aliases
pub const FLAG_Z: u8 = 0b10000000;
pub const FLAG_N: u8 = 0b01000000;
pub const FLAG_H: u8 = 0b00100000;
pub const FLAG_C: u8 = 0b00010000;

// IO register aliases
pub const IO_IE: usize = 0xff;

pub struct Gameboy {
    pub mem: Box<[u8; 0xffff]>,
    /// Elapsed machine cycles
    pub cycles: i64, 
    pub pc: u16,
    pub sp: u16,
    /// Registers A, B, C, D, E, F, H, L
    pub regs: [u8; 8], 
    /// Registers 0xFF00 to 0xFFFF
    pub io_regs: [AtomicU8; 256],
    /// Interrupt Master Enable
    pub ime: bool,
    pub halted: bool,
    pub stopped: bool,
}

impl Gameboy {
    pub fn new() -> Gameboy {
        let mut io_regs = vec!();
        for _ in 0..256 {
            io_regs.push(AtomicU8::new(0));
        }
        Gameboy {
            mem: Box::new([0; 0xffff]),
            cycles: 0,
            pc: 0x0100, 
            sp: 0xfffe,
            regs: [0; 8],
            io_regs: io_regs.try_into().unwrap(),
            ime: false,
            halted: false,
            stopped: false,
        }
    }
}

// TODO probably should move this somewhere, or make it a plain function
impl fmt::Display for Gameboy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, 
            concat!(
                "PC: {:0>4X}, SP: {:0>X}, cycles: {:0>6}\n",
                "Z: {:2}, N: {:2}, H: {:2}, C: {:2}\n",
                "A: {:0>2X}, B: {:0>2X}, D: {:0>2X}, H: {:0>2X}\n",
                "F: {:0>2X}, C: {:0>2X}, E: {:0>2X}, L: {:0>2X}",
            ),
            self.pc, self.sp, self.cycles,
            (self.regs[RF] & FLAG_Z) >> 7, (self.regs[RF] & FLAG_N) >> 6,
            (self.regs[RF] & FLAG_H) >> 5, (self.regs[RF] & FLAG_C) >> 4,
            self.regs[RA], self.regs[RB], self.regs[RD], self.regs[RH],
            self.regs[RF], self.regs[RC], self.regs[RE], self.regs[RL],
        )
    }
}

/// Convert a register pair alias into a u16
pub fn rr_to_u16(gb: &Gameboy, reg_pair: RR) -> u16 {
    let upper = (gb.regs[reg_pair.0] as u16) << 8;
    let lower = gb.regs[reg_pair.1] as u16;
    upper ^ lower
}
