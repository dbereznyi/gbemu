use std::num::Wrapping;
use crate::gameboy::gameboy::{*};

#[derive(Debug, Copy, Clone)]
pub enum CarryMode {
    NoCarry,
    WithCarry,
}

#[derive(Debug, Copy, Clone)]
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
    pub fn read(&self, gb: &Gameboy) -> u8 {
        match *self {
            Src8::R8(r) => gb.regs[r],
            Src8::Id(rr) => gb.read(rr_to_u16(gb, rr)),
            Src8::IdFFRC => gb.read(0xff00 | (gb.regs[RC] as u16)),
            Src8::IdFF(n) => gb.read(0xff00 | (n as u16)),
            Src8::IdNN(nn) => gb.read(nn),
            Src8::D8(n) => n,
        }
    }
}

#[derive(Debug, Copy, Clone)]
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
    pub fn read(&self, gb: &Gameboy) -> u8 {
        match *self {
            Dst8::R8(r) => gb.regs[r],
            Dst8::Id(rr) => gb.read(rr_to_u16(gb, rr)),
            Dst8::IdFFRC => gb.read(0xff00 | (gb.regs[RC] as u16)),
            Dst8::IdFF(n) => gb.read(0xff00 | (n as u16)),
            Dst8::IdNN(nn) => gb.read(nn),
        }
    }

    pub fn write(&self, gb: &mut Gameboy, value: u8) {
        match *self {
            Dst8::R8(r) => gb.regs[r] = value,
            Dst8::Id(rr) => gb.write(rr_to_u16(gb, rr), value),
            Dst8::IdFFRC => gb.write(0xff00 | (gb.regs[RC] as u16), value),
            Dst8::IdFF(n) => gb.write(0xff00 | (n as u16), value),
            Dst8::IdNN(nn) => gb.write(nn, value),
        }
    }
}

#[derive(Debug, Copy, Clone)]
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
    pub fn read(&self, gb: &Gameboy) -> u16 {
        match *self {
            Src16::R16(rr) => rr_to_u16(gb, rr),
            Src16::RSP => gb.sp,
            Src16::D16(nn) => nn,
            Src16::SPD8(n) => ((Wrapping(gb.sp as i16) + Wrapping(n as i16)).0) as u16,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Dst16 {
    /// A 16-bit register.
    R16(RR),
    /// The stack pointer register.
    RSP,
    /// Indirect, address specified by immediate 16-bit value.
    IdNN(u16),
}

impl Dst16 {
    pub fn read(&self, gb: &Gameboy) -> u16 {
        match *self {
            Dst16::R16(rr) => rr_to_u16(gb, rr),
            Dst16::RSP => gb.sp,
            Dst16::IdNN(nn) => {
                let high = (gb.read(nn) as u16) << 8;
                let low = gb.read(nn + 1) as u16;
                high | low
            },
        }
    }

    pub fn write(&self, gb: &mut Gameboy, value: u16) {
        match *self {
            Dst16::R16(rr) => {
                gb.regs[rr.0] = (value >> 8) as u8;
                gb.regs[rr.1] = value as u8;
            },
            Dst16::RSP => gb.sp = value,
            Dst16::IdNN(nn) => {
                gb.write(nn, (gb.sp >> 8) as u8);
                gb.write(nn + 1, gb.sp as u8);
            },
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum BitwiseOp {
    And, Xor, Or,
}

#[derive(Debug, Copy, Clone)]
pub enum IncDec {
    Inc, Dec
}

#[derive(Debug, Copy, Clone)]
pub enum AddSub {
    Add, Sub
}

#[derive(Debug, Copy, Clone)]
pub enum Cond {
    Z,
    Nz,
    C,
    Nc,
}

impl Cond {
    pub fn check(&self, gb: &Gameboy) -> bool {
        match *self {
            Cond::Z => ((gb.regs[RF] & FLAG_Z) >> 7) == 1,
            Cond::Nz => ((gb.regs[RF] & FLAG_Z) >> 7) == 0,
            Cond::C => ((gb.regs[RF] & FLAG_C) >> 4) == 1,
            Cond::Nc => ((gb.regs[RF] & FLAG_C) >> 4) == 0,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub enum Instr {
    // Control/misc
    Nop,
    Stop,
    Halt,
    Di,
    Ei,
    Ccf,
    Scf,
    Daa,
    Cpl,
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
    // Control-flow
    Jp(Src16),
    JpCC(Cond, u16),
    Jr(i8),
    JrCC(Cond, i8),
    Call(u16),
    CallCC(Cond, u16),
    Ret,
    RetCC(Cond),
    Reti,
    Rst(u8),
    // Rotates, shifts, bit operations
    Rlca,
    Rla,
    Rrca,
    Rra,
    Rlc(Dst8),
    Rrc(Dst8),
    Rl(Dst8),
    Rr(Dst8),
    Sla(Dst8),
    Sra(Dst8),
    Srl(Dst8),
    Bit(u8, Dst8),
    Res(u8, Dst8),
    Set(u8, Dst8),
    Swap(Dst8),
}

impl Instr {
    /// The number of machine cycles an instruction takes to execute.
    pub fn num_cycles(&self, gb: &Gameboy) -> u64 {
        match self {
            Instr::Nop => 1,
            Instr::Stop => 1,
            Instr::Halt => 1,
            Instr::Di => 1,
            Instr::Ei => 1,
            Instr::Ccf => 1,
            Instr::Scf => 1,
            Instr::Daa => 1,
            Instr::Cpl => 1,

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
            Instr::LdInc(_, _) | Instr::LdDec(_, _) => 2,

            Instr::Ld16(dst, src) => match (dst, src) {
                (Dst16::RSP, Src16::R16(RHL)) => 2,
                (Dst16::RSP, Src16::D16(_)) => 3,
                (Dst16::R16(_), Src16::D16(_)) => 3,
                (Dst16::R16(RHL), Src16::SPD8(_)) => 3,
                (Dst16::IdNN(_), Src16::RSP) => 5,
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

            Instr::Jp(src) => match src {
                Src16::D16(_) => 4,
                Src16::R16(RHL) => 1,
                _ => panic!("Invalid dst, src"),
            },
            Instr::JpCC(cond, _) => if cond.check(gb) { 4 } else { 3 },
            Instr::Jr(_) => 3,
            Instr::JrCC(cond, _) => if cond.check(gb) { 3 } else { 2 },
            Instr::Call(_) => 6,
            Instr::CallCC(cond, _) => if cond.check(gb) { 6 } else { 3 },
            Instr::Ret => 4,
            Instr::RetCC(cond) => if cond.check(gb) { 5 } else { 2 },
            Instr::Reti => 4,
            Instr::Rst(_) => 4,

            Instr::Rlca | Instr::Rla | Instr::Rrca | Instr::Rra => 1,
            Instr::Rlc(Dst8::Id(RHL)) => 4,
            Instr::Rlc(_) => 2,
            Instr::Rrc(Dst8::Id(RHL)) => 4,
            Instr::Rrc(_) => 2,
            Instr::Rl(Dst8::Id(RHL)) => 4,
            Instr::Rl(_) => 2,
            Instr::Rr(Dst8::Id(RHL)) => 4,
            Instr::Rr(_) => 2,
            Instr::Sla(Dst8::Id(RHL)) => 4,
            Instr::Sla(_) => 2,
            Instr::Sra(Dst8::Id(RHL)) => 4,
            Instr::Sra(_) => 2,
            Instr::Srl(Dst8::Id(RHL)) => 4,
            Instr::Srl(_) => 2,
            Instr::Bit(_, Dst8::Id(RHL)) => 3,
            Instr::Bit(_, _) => 2,
            Instr::Res(_, Dst8::Id(RHL)) => 4,
            Instr::Res(_, _) => 2,
            Instr::Set(_, Dst8::Id(RHL)) => 4,
            Instr::Set(_, _) => 2,
            Instr::Swap(Dst8::Id(RHL)) => 4,
            Instr::Swap(_) => 2,
        }
    }

    /// The length, in bytes, of an instruction. Used to calculate next PC value.
    /// For jump instructions, 0 is returned if PC would be directly modified by the instruction.
    pub fn size(&self, gb: &Gameboy) -> u16 {
        match self {
            Instr::Nop => 1,
            Instr::Stop => 2,
            Instr::Halt => 1,
            Instr::Di => 1,
            Instr::Ei => 1,
            Instr::Ccf => 1,
            Instr::Scf => 1,
            Instr::Daa => 1,
            Instr::Cpl => 1,

            Instr::Ld(dst, src) => match (dst, src) {
                (Dst8::R8(_), Src8::R8(_)) => 1,
                (Dst8::R8(_), Src8::D8(_)) => 2,
                (Dst8::Id(_), Src8::R8(_)) | (Dst8::R8(_), Src8::Id(_)) => 1,
                (Dst8::IdFFRC, Src8::R8(RA)) | (Dst8::R8(RA), Src8::IdFFRC) => 1,
                (Dst8::IdFF(_), Src8::R8(RA)) | (Dst8::R8(RA), Src8::IdFF(_)) => 2,
                (Dst8::Id(RHL), Src8::D8(_)) => 2,
                (Dst8::IdNN(_), Src8::R8(RA)) | (Dst8::R8(RA), Src8::IdNN(_)) => 3,
                _ => panic!("Invalid dst, src"),
            },
            Instr::LdInc(_, _) | Instr::LdDec(_, _) => 1,

            Instr::Ld16(_, src) => match src {
                Src16::R16(RHL) => 1,
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

            Instr::Jp(_) => 0,
            Instr::JpCC(cond, _) => if cond.check(gb) { 0 } else { 3 },
            Instr::Jr(_) => 0,
            Instr::JrCC(cond, _) => if cond.check(gb) { 0 } else { 3 },
            Instr::Call(_) => 0,
            Instr::CallCC(cond, _) => if cond.check(gb) { 0 } else { 3 },
            Instr::Ret => 0,
            Instr::RetCC(cond) => if cond.check(gb) { 0 } else { 3 },
            Instr::Reti => 0,
            Instr::Rst(_) => 0,

            Instr::Rlca | Instr::Rla | Instr::Rrca | Instr::Rra => 1,
            Instr::Rlc(_) => 2,
            Instr::Rrc(_) => 2,
            Instr::Rl(_) => 2,
            Instr::Rr(_) => 2,
            Instr::Sla(_) => 2,
            Instr::Sra(_) => 2,
            Instr::Srl(_) => 2,
            Instr::Bit(_, _) => 2,
            Instr::Res(_, _) => 2,
            Instr::Set(_, _) => 2,
            Instr::Swap(_) => 2,
        }
    }
}