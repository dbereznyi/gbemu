use std::num::Wrapping;
use super::super::gameboy::{*};

pub enum CarryMode {
    NoCarry,
    WithCarry,
}

/// A source value used to compute the result of an instruction.
pub enum Src8 {
    /// An 8-bit register.
    R8(R),
    /// Indirect, address specified by register pair.
    Id(RR),
    /// Indirect, address specified by 0xFF00 | C.
    IdFFRC,
    /// Indirect, address specified by 0xFF00 | immediate 8-bit value.
    IdFF(u8),
    /// Indirect, address specified by immediate 16-bit value.
    IdNN(u16),
    /// An immediate 8-bit value.
    D8(u8),
}

impl Src8 {
    pub fn get_value(gb: &Gameboy, src: &Src8) -> u8 {
        match *src {
            Src8::R8(r) => gb.regs[r],
            Src8::Id(rr) => gb.mem[rr_to_u16(gb, rr) as usize],
            Src8::IdFFRC => gb.mem[(0xFF00 | (gb.regs[RC] as u16)) as usize],
            Src8::IdFF(n) => gb.mem[(0xFF00 | (n as u16)) as usize],
            Src8::IdNN(nn) => gb.mem[nn as usize],
            Src8::D8(n) => n,
        }
    }
}

/// A destination to store the result of an instruction.
pub enum Dst8 {
    /// An 8-bit register.
    R8(R),
    /// Indirect, address specified by register pair.
    Id(RR),
    /// Indirect, address specified by 0xFF00 | C.
    IdFFRC,
    /// Indirect, address specified by 0xFF00 | immediate 8-bit value.
    IdFF(u8),
    /// Indirect, address specified by immediate 16-bit value.
    IdNN(u16),
}

impl Dst8 {
    pub fn get_value(gb: &Gameboy, dst: &Dst8) -> u8 {
        match *dst {
            Dst8::R8(r) => gb.regs[r],
            Dst8::Id(rr) => gb.mem[rr_to_u16(gb, rr) as usize],
            Dst8::IdFFRC => gb.mem[(0xFF00 | (gb.regs[RC] as u16)) as usize],
            Dst8::IdFF(n) => gb.mem[(0xFF00 | (n as u16)) as usize],
            Dst8::IdNN(nn) => gb.mem[nn as usize],
        }
    }

    pub fn set_value(gb: &mut Gameboy, dst: &Dst8, value: u8) {
        match *dst {
            Dst8::R8(r) => gb.regs[r] = value,
            Dst8::Id(rr) => gb.mem[rr_to_u16(gb, rr) as usize] = value,
            Dst8::IdFFRC => gb.mem[(0xFF00 | (gb.regs[RC] as u16)) as usize] = value,
            Dst8::IdFF(n) => gb.mem[(0xFF00 | (n as u16)) as usize] = value,
            Dst8::IdNN(nn) => gb.mem[nn as usize] = value,
        }
    }
}

pub enum Src16 {
    /// A 16-bit register.
    R16(RR),
    /// The stack pointer register.
    RSP,
    /// An immediate 16-bit value.
    D16(u16),
    /// SP + 8-bit immediate value.
    SPD8(i8),
}

impl Src16 {
    pub fn get_value(gb: &Gameboy, src: &Src16) -> u16 {
        match *src {
            Src16::R16(rr) => rr_to_u16(gb, rr),
            Src16::RSP => gb.sp,
            Src16::D16(nn) => nn,
            Src16::SPD8(n) => ((Wrapping(gb.sp as i16) + Wrapping(n as i16)).0) as u16,
        }
    }
}

pub enum Dst16 {
    /// A 16-bit register.
    R16(RR),
    /// The stack pointer register.
    RSP,
    /// Indirect, address specified by immediate 16-bit value.
    IdNN(u16),
}

impl Dst16 {
    pub fn get_value(gb: &Gameboy, dst: &Dst16) -> u16 {
        match *dst {
            Dst16::R16(rr) => rr_to_u16(gb, rr),
            Dst16::RSP => gb.sp,
            Dst16::IdNN(nn) => 
                ((gb.mem[nn as usize] as u16) << 8) | (gb.mem[(nn + 1) as usize] as u16),
        }
    }

    pub fn set_value(gb: &mut Gameboy, dst: &Dst16, value: u16) {
        match *dst {
            Dst16::R16(rr) => {
                gb.regs[rr.0] = (value >> 8) as u8;
                gb.regs[rr.1] = value as u8;
            },
            Dst16::RSP => gb.sp = value,
            Dst16::IdNN(nn) => {
                gb.mem[nn as usize] = (gb.sp >> 8) as u8;
                gb.mem[(nn + 1) as usize] = gb.sp as u8;
            },
        }
    }
}

pub enum BitwiseOp {
    And, Xor, Or,
}

pub enum IncDec {
    Inc, Dec
}

pub enum AddSub {
    Add, Sub
}

pub enum Instr {
    // Control/misc
    Nop,
    Stop,
    Halt,
    // 8-bit load
    Ld(Dst8, Src8),
    LdInc(Dst8, Src8),
    LdDec(Dst8, Src8),
    // 16-bit load
    Ld16(Dst16, Src16),
    Push(RR),
    Pop(RR),
    // 8-bit arithmetic
    Inc(Dst8),
    Dec(Dst8),
    Add(Src8),
    Adc(Src8),
    Sub(Src8),
    Sbc(Src8),
    And(Src8),
    Xor(Src8),
    Or(Src8),
    Cp(Src8),
    // 16-bit arithmetic
    Add16HL(Src16),
    Add16SP(i8),
    Inc16(Dst16),
    Dec16(Dst16),
}

impl Instr {
    /// The number of machine cycles an instruction takes to execute.
    pub fn num_cycles(instr: &Instr) -> i64 {
        match instr {
            Instr::Nop => 1,
            Instr::Stop => 1,
            Instr::Halt => 1,

            Instr::Ld(dst, src) => match (dst, src) {
                (Dst8::R8(_), Src8::R8(_)) => 1,
                (Dst8::R8(_), Src8::D8(_)) => 2,
                (Dst8::Id(_), Src8::R8(_)) | (Dst8::R8(_), Src8::Id(_)) => 2,
                (Dst8::IdFFRC, Src8::R8(RA)) | (Dst8::R8(RA), Src8::IdFFRC) => 2,
                (Dst8::IdFF(_), Src8::R8(RA)) | (Dst8::R8(RA), Src8::IdFF(_)) => 3,
                (Dst8::Id(RHL), Src8::D8(_)) => 3,
                (Dst8::IdNN(_), Src8::R8(RA)) | (Dst8::R8(RA), Src8::IdNN(_)) => 4,
                _ => panic!("Invalid dst, src"),
            },
            Instr::LdInc(dst, _) | Instr::LdDec(dst, _) => match dst {
                Dst8::R8(_) => 1,
                _ => 3,
            },

            Instr::Ld16(dst, src) => match (dst, src) {
                (Dst16::R16(_), Src16::D16(_)) | (Dst16::RSP, Src16::D16(_)) => 3,
                (Dst16::IdNN(_), Src16::RSP) => 5,
                (Dst16::RSP, Src16::R16(_)) => 3,
                _ => panic!("Invalid dst, src"),
            },

            Instr::Push(_) => 4,
            Instr::Pop(_) => 3,

            Instr::Inc(dst) | Instr::Dec(dst) => match dst {
                Dst8::R8(_) => 1,
                _ => 3,
            },
            Instr::Add(src) | Instr::Adc(src) | Instr::Sub(src) | Instr::Sbc(src) 
            | Instr::And(src) | Instr::Xor(src) | Instr::Or(src) | Instr::Cp(src) 
            => match src {
                Src8::R8(_) => 1,
                _ => 2,
            },

            Instr::Add16HL(_) => 2,
            Instr::Add16SP(_) => 4,
            Instr::Inc16(_) | Instr::Dec16(_) => 2,
        }
    }

    /// The length, in bytes, of an instruction.
    pub fn size(instr: &Instr) -> u16 {
        match instr {
            Instr::Nop => 1,
            Instr::Stop => 2,
            Instr::Halt => 1,

            Instr::Ld(dst, src) => match (dst, src) {
                (Dst8::R8(_), Src8::R8(_)) => 1,
                (Dst8::R8(_), Src8::D8(_)) => 2,
                (Dst8::Id(_), Src8::R8(_)) | (Dst8::R8(_), Src8::Id(_)) => 1,
                (Dst8::IdFFRC, Src8::R8(RA)) | (Dst8::R8(RA), Src8::IdFFRC) => 2,
                (Dst8::IdFF(_), Src8::R8(RA)) | (Dst8::R8(RA), Src8::IdFF(_)) => 2,
                (Dst8::Id(RHL), Src8::D8(_)) => 2,
                (Dst8::IdNN(_), Src8::R8(RA)) | (Dst8::R8(RA), Src8::IdNN(_)) => 3,
                _ => panic!("Invalid dst, src"),
            },
            Instr::LdInc(_, _) | Instr::LdDec(_, _) => 1,

            Instr::Ld16(_, src) => match src {
                Src16::SPD8(_) => 2,
                _ => 3,
            },

            Instr::Push(_) | Instr::Pop(_) => 1,

            Instr::Inc(_) | Instr::Dec(_) => 1,
            Instr::Add(src) | Instr::Adc(src) | Instr::Sub(src) | Instr::Sbc(src) 
            | Instr::And(src) | Instr::Xor(src) | Instr::Or(src) | Instr::Cp(src) 
            => match src {
                Src8::D8(_) => 2,
                _ => 1,
            },

            Instr::Add16HL(_) => 1,
            Instr::Add16SP(_) => 2,
            Instr::Inc16(_) | Instr::Dec16(_) => 1,
        }
    }
}