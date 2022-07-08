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

    pub fn to_str(&self) -> String {
        fn reg8_to_str(r: R) -> &'static str {
            match r {
                RA => "a",
                RB => "b",
                RC => "c",
                RD => "d",
                RE => "e",
                RF => "f",
                _ => panic!("Invalid 8-bit register {}", r),
            }
        }
        fn reg16_to_str(rr: RR) -> &'static str {
            match rr {
                RAF => "af",
                RBC => "bc",
                RDE => "de",
                RHL => "hl",
                _ => panic!("Invalid 16-bit register {:?}", rr),
            }
        }
        fn dst8(s: &mut String, dst: Dst8, inc_dec: Option<IncDec>) {
            match dst {
                Dst8::R8(r) => s.push_str(reg8_to_str(r)),
                Dst8::Id(rr) => {
                    s.push_str(format!("(${})", reg16_to_str(rr)).as_str());
                    match inc_dec {
                       Some(IncDec::Inc) => s.push('+'),
                       Some(IncDec::Dec) => s.push('-'),
                       _ => (),
                    }
                },
                Dst8::IdFFRC => s.push_str("(C)"),
                Dst8::IdFF(n) => s.push_str(format!("(${:0>2x})", n).as_str()),
                Dst8::IdNN(nn) => s.push_str(format!("(${:0>4x})", nn).as_str()),
            }
        }
        fn src8(s: &mut String, src: Src8, inc_dec: Option<IncDec>) {
            match src {
                Src8::R8(r) => s.push_str(reg8_to_str(r)),
                Src8::Id(rr) => {
                    s.push_str(format!("(${})", reg16_to_str(rr)).as_str());
                    match inc_dec {
                       Some(IncDec::Inc) => s.push('+'),
                       Some(IncDec::Dec) => s.push('-'),
                       _ => (),
                    }
                },
                Src8::IdFFRC => s.push_str("(C)"),
                Src8::IdFF(n) => s.push_str(format!("(${:0>2x})", n).as_str()),
                Src8::IdNN(nn) => s.push_str(format!("(${:0>4x})", nn).as_str()),
                Src8::D8(n) => s.push_str(format!("${:0>2x}", n).as_str()),
            }
        }
        fn dst16(s: &mut String, dst: Dst16) {
            match dst {
                Dst16::R16(rr) => s.push_str(reg16_to_str(rr)),
                Dst16::RSP => s.push_str("sp"),
                Dst16::IdNN(nn) => s.push_str(format!("(${:0>4x})", nn).as_str()),
            }
        }
        fn src16(s: &mut String, src: Src16) {
            match src {
                Src16::R16(rr) => s.push_str(reg16_to_str(rr)),
                Src16::RSP => s.push_str("sp"),
                Src16::D16(nn) => s.push_str(format!("${:0>4x}", nn).as_str()),
                Src16::SPD8(n) => s.push_str(format!("sp + ${:0>2x}", n).as_str()),
            }
        }
        fn r8(s: &mut String, n: i8) {
            s.push_str(format!("${:0>2x}", n).as_str());
        }
        fn cond(s: &mut String, c: Cond) {
            match c {
                Cond::Z => s.push('z'),
                Cond::Nz => s.push_str("nz"),
                Cond::C => s.push('c'),
                Cond::Nc => s.push_str("nc"),
            }
        }

        match *self {
            Instr::Nop => String::from("nop"),
            Instr::Stop => String::from("stop"),
            Instr::Halt => String::from("halt"),
            Instr::Di => String::from("di"),
            Instr::Ei => String::from("ei"),
            Instr::Ccf => String::from("ccf"),
            Instr::Scf => String::from("scf"),
            Instr::Daa => String::from("daa"),
            Instr::Cpl => String::from("cpl"),
            Instr::Ld(dst, src) => {
                let mut s = String::from("ld");
                match dst {
                    Dst8::IdFFRC | Dst8::IdFF(_) => s.push('h'),
                    _ => (),
                };
                match src {
                    Src8::IdFFRC | Src8::IdFF(_) => s.push('h'),
                    _ => (),
                };
                s.push(' ');
                dst8(&mut s, dst, None);
                s.push_str(", ");
                src8(&mut s, src, None);
                s
            },
            Instr::LdInc(dst, src) => {
                let mut s = String::from("ld ");
                dst8(&mut s, dst, Some(IncDec::Inc));
                s.push_str(", ");
                src8(&mut s, src, Some(IncDec::Inc));
                s
            },
            Instr::LdDec(dst, src) => {
                let mut s = String::from("ld ");
                dst8(&mut s, dst, Some(IncDec::Dec));
                s.push_str(", ");
                src8(&mut s, src, Some(IncDec::Dec));
                s
            },
            Instr::Ld16(dst, src) => {
                let mut s = String::from("ld ");
                dst16(&mut s, dst);
                s.push_str(", ");
                src16(&mut s, src);
                s
            },
            Instr::Push(rr) => {
                let mut s = String::from("push ");
                src16(&mut s, Src16::R16(rr));
                s
            },
            Instr::Pop(rr) => {
                let mut s = String::from("pop ");
                src16(&mut s, Src16::R16(rr));
                s
            },
            Instr::Inc(dst) => {
                let mut s = String::from("inc ");
                dst8(&mut s, dst, None);
                s
            },
            Instr::Dec(dst) => {
                let mut s = String::from("dec ");
                dst8(&mut s, dst, None);
                s
            },
            Instr::Add(src) => {
                let mut s = String::from("add ");
                src8(&mut s, src, None);
                s
            },
            Instr::Adc(src) => {
                let mut s = String::from("adc ");
                src8(&mut s, src, None);
                s
            },
            Instr::Sub(src) => {
                let mut s = String::from("sub ");
                src8(&mut s, src, None);
                s
            },
            Instr::Sbc(src) => {
                let mut s = String::from("sbc ");
                src8(&mut s, src, None);
                s
            },
            Instr::And(src) => {
                let mut s = String::from("and ");
                src8(&mut s, src, None);
                s
            },
            Instr::Xor(src) => {
                let mut s = String::from("xor ");
                src8(&mut s, src, None);
                s
            },
            Instr::Or(src) => {
                let mut s = String::from("or ");
                src8(&mut s, src, None);
                s
            },
            Instr::Cp(src) => {
                let mut s = String::from("cp ");
                src8(&mut s, src, None);
                s
            },
            Instr::Add16HL(src) => {
                let mut s = String::from("add hl, ");
                src16(&mut s, src);
                s
            },
            Instr::Add16SP(n) => {
                let mut s = String::from("add sp, ");
                r8(&mut s, n);
                s
            },
            Instr::Inc16(dst) => {
                let mut s = String::from("inc ");
                dst16(&mut s, dst);
                s
            },
            Instr::Dec16(dst) => {
                let mut s = String::from("dec ");
                dst16(&mut s, dst);
                s
            },
            Instr::Jp(src) => {
                let mut s = String::from("jp ");
                src16(&mut s, src);
                s
            },
            Instr::JpCC(c, nn) => {
                let mut s = String::from("jp ");
                cond(&mut s, c);
                s.push_str(", ");
                src16(&mut s, Src16::D16(nn));
                s
            },
            Instr::Jr(n) => {
                let mut s = String::from("jr ");
                r8(&mut s, n);
                s
            },
            Instr::JrCC(c, n) => {
                let mut s = String::from("jr ");
                cond(&mut s, c);
                s.push_str(", ");
                r8(&mut s, n);
                s
            },
            Instr::Call(nn) => {
                let mut s = String::from("call ");
                src16(&mut s, Src16::D16(nn));
                s
            },
            Instr::CallCC(c, nn) => {
                let mut s = String::from("call ");
                cond(&mut s, c);
                s.push_str(", ");
                src16(&mut s, Src16::D16(nn));
                s
            },
            Instr::Ret => String::from("ret"),
            Instr::RetCC(c) => {
                let mut s = String::from("ret ");
                cond(&mut s, c);
                s
            },
            Instr::Reti => String::from("reti"),
            Instr::Rst(n) => {
                let mut s = String::from("rst ");
                src8(&mut s, Src8::D8(n), None);
                s
            },
            Instr::Rlca => String::from("rlca"),
            Instr::Rla => String::from("rla"),
            Instr::Rrca => String::from("rrca"),
            Instr::Rra => String::from("rra"),
            Instr::Rlc(dst) => {
                let mut s = String::from("rlc ");
                dst8(&mut s, dst, None);
                s
            },
            Instr::Rrc(dst) => {
                let mut s = String::from("rrc ");
                dst8(&mut s, dst, None);
                s
            },
            Instr::Rl(dst) => {
                let mut s = String::from("rl ");
                dst8(&mut s, dst, None);
                s
            },
            Instr::Rr(dst) => {
                let mut s = String::from("rr ");
                dst8(&mut s, dst, None);
                s
            },
            Instr::Sla(dst) => {
                let mut s = String::from("sla ");
                dst8(&mut s, dst, None);
                s
            },
            Instr::Sra(dst) => {
                let mut s = String::from("sra ");
                dst8(&mut s, dst, None);
                s
            },
            Instr::Srl(dst) => {
                let mut s = String::from("srl ");
                dst8(&mut s, dst, None);
                s
            },
            Instr::Bit(n, dst) => {
                let mut s = String::from("bit ");
                s.push_str(format!("{}", n).as_str());
                s.push_str(", ");
                dst8(&mut s, dst, None);
                s
            },
            Instr::Res(n, dst) => {
                let mut s = String::from("res ");
                s.push_str(format!("{}", n).as_str());
                s.push_str(", ");
                dst8(&mut s, dst, None);
                s
            },
            Instr::Set(n, dst) => {
                let mut s = String::from("set ");
                s.push_str(format!("{}", n).as_str());
                s.push_str(", ");
                dst8(&mut s, dst, None);
                s
            },
            Instr::Swap(dst) => {
                let mut s = String::from("swap ");
                dst8(&mut s, dst, None);
                s
            },
        }
    }
}

