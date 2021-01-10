use super::super::gameboy::{*};
use super::exec::{*};
use super::instruction::{CarryMode, Src8, Dst8, Src16, Dst16, BitwiseOp, IncDec, AddSub, Instr};

// Currently simulating a single instruction at a time
// Woud be more accurate to simulate each machine cycle one by one instead, but may not be
// necessary. Will see.

/// Decode the instruction at PC, then execute it and update PC and cycle count accordingly.
pub fn step(gb: &mut Gameboy) {
    let instr = decode(gb);
    match &instr {
        // Control/misc
        Instr::Nop             => (),
        Instr::Stop            => stop(gb),
        Instr::Halt            => halt(gb),
        // 8-bit load
        Instr::Ld(dst, src)    => ld(gb, dst, src),
        Instr::LdInc(dst, src) => ld_inc_dec(gb, dst, src, IncDec::Inc),
        Instr::LdDec(dst, src) => ld_inc_dec(gb, dst, src, IncDec::Dec),
        // 16-bit load
        Instr::Ld16(dst, src)  => ld_16(gb, dst, src),
        Instr::Push(r_pair)    => push(gb, r_pair),
        Instr::Pop(r_pair)     => pop(gb, r_pair),
        // 8-bit arithmetic
        Instr::Inc(dst)        => inc_dec(gb, dst, IncDec::Inc),
        Instr::Dec(dst)        => inc_dec(gb, dst, IncDec::Dec),
        Instr::Add(src)        => add_sub(gb, src, AddSub::Add, CarryMode::NoCarry),
        Instr::Adc(src)        => add_sub(gb, src, AddSub::Add, CarryMode::WithCarry),
        Instr::Sub(src)        => add_sub(gb, src, AddSub::Sub, CarryMode::NoCarry),
        Instr::Sbc(src)        => add_sub(gb, src, AddSub::Sub, CarryMode::WithCarry),
        Instr::And(src)        => bitwise(gb, src, BitwiseOp::And),
        Instr::Xor(src)        => bitwise(gb, src, BitwiseOp::Xor),
        Instr::Or(src)         => bitwise(gb, src, BitwiseOp::Or),
        Instr::Cp(src)         => cp(gb, src),
        // 16-bit arithmetic
        Instr::Add16HL(src)    => add_16_hl(gb, src),
        Instr::Add16SP(n)      => {
            let value = Src16::get_value(gb, &Src16::SPD8(*n));
            Dst16::set_value(gb, &Dst16::RSP, value);
        },
        Instr::Inc16(dst)      => inc_dec_16(gb, dst, IncDec::Inc),
        Instr::Dec16(dst)      => inc_dec_16(gb, dst, IncDec::Dec),
    }
    // TODO Conditional instructions need to check the flags to compute cycles

    gb.pc += Instr::size(&instr);
    gb.cycles += Instr::num_cycles(&instr);
}

/// Decode the current instruction.
fn decode(gb: &mut Gameboy) -> Instr {
    let opcode = gb.mem[gb.pc as usize];
    // The bottom three bits of the opcode are used to indicate src reg for certain loads
    let r_src = (opcode & 0b00000111) as usize;
    // Grab data from PC+1 and PC+2 in case we need them as arguments
    // This shouldn't go out of bounds since instructions aren't executed in top of mem
    let n = gb.mem[(gb.pc + 1) as usize];
    let n2 = gb.mem[(gb.pc + 2) as usize];
    let nn = ((n as u16) << 8) & (n2 as u16);
    match opcode {
        0x00        => Instr::Nop,
        0x01        => Instr::Ld16(Dst16::R16(RBC), Src16::D16(nn)),
        0x02        => Instr::Ld(Dst8::Id(RBC), Src8::R8(RA)),
        0x03        => Instr::Inc16(Dst16::R16(RBC)),
        0x04        => Instr::Inc(Dst8::R8(RB)),
        0x05        => Instr::Dec(Dst8::R8(RB)),
        0x06        => Instr::Ld(Dst8::R8(RB), Src8::D8(n)),
        0x08        => Instr::Ld16(Dst16::IdNN(nn), Src16::RSP),
        0x09        => Instr::Add16HL(Src16::R16(RBC)),
        0x0a        => Instr::Ld(Dst8::R8(RA), Src8::Id(RBC)),
        0x0b        => Instr::Dec16(Dst16::R16(RBC)),
        0x0c        => Instr::Inc(Dst8::R8(RC)),
        0x0d        => Instr::Dec(Dst8::R8(RC)),
        0x0e        => Instr::Ld(Dst8::R8(RC), Src8::D8(n)),

        0x10        => Instr::Stop,
        0x11        => Instr::Ld16(Dst16::R16(RDE), Src16::D16(nn)),
        0x12        => Instr::Ld(Dst8::Id(RDE), Src8::R8(RA)),
        0x13        => Instr::Inc16(Dst16::R16(RDE)),
        0x14        => Instr::Inc(Dst8::R8(RD)),
        0x15        => Instr::Dec(Dst8::R8(RD)),
        0x16        => Instr::Ld(Dst8::R8(RD), Src8::D8(n)),
        0x19        => Instr::Add16HL(Src16::R16(RDE)),
        0x1a        => Instr::Ld(Dst8::R8(RA), Src8::Id(RDE)),
        0x1b        => Instr::Dec16(Dst16::R16(RDE)),
        0x1c        => Instr::Inc(Dst8::R8(RE)),
        0x1d        => Instr::Dec(Dst8::R8(RE)),
        0x1e        => Instr::Ld(Dst8::R8(RE), Src8::D8(n)),

        0x21        => Instr::Ld16(Dst16::R16(RHL), Src16::D16(nn)),
        0x22        => Instr::LdInc(Dst8::Id(RHL), Src8::R8(RA)),
        0x23        => Instr::Inc16(Dst16::R16(RHL)),
        0x24        => Instr::Inc(Dst8::R8(RH)),
        0x25        => Instr::Dec(Dst8::R8(RH)),
        0x26        => Instr::Ld(Dst8::R8(RH), Src8::D8(n)),
        0x29        => Instr::Add16HL(Src16::R16(RHL)),
        0x2a        => Instr::LdInc(Dst8::R8(RA), Src8::Id(RHL)),
        0x2b        => Instr::Inc16(Dst16::R16(RHL)),
        0x2c        => Instr::Inc(Dst8::R8(RL)),
        0x2d        => Instr::Dec(Dst8::R8(RL)),
        0x2e        => Instr::Ld(Dst8::R8(RL), Src8::D8(n)),

        0x31        => Instr::Ld16(Dst16::RSP, Src16::D16(nn)),
        0x32        => Instr::LdDec(Dst8::Id(RHL), Src8::R8(RA)),
        0x33        => Instr::Inc16(Dst16::RSP),
        0x34        => Instr::Inc(Dst8::Id(RHL)),
        0x35        => Instr::Dec(Dst8::Id(RHL)),
        0x36        => Instr::Ld(Dst8::Id(RHL), Src8::D8(n)),
        0x39        => Instr::Add16HL(Src16::RSP),
        0x3a        => Instr::LdDec(Dst8::R8(RA), Src8::Id(RHL)),
        0x3b        => Instr::Dec16(Dst16::RSP),
        0x3c        => Instr::Inc(Dst8::R8(RA)),
        0x3d        => Instr::Dec(Dst8::R8(RA)),
        0x3e        => Instr::Ld(Dst8::R8(RA), Src8::D8(n)),

        0x40..=0x45 => Instr::Ld(Dst8::R8(RB), Src8::R8(r_src)),
        0x46        => Instr::Ld(Dst8::R8(RB), Src8::Id(RHL)),
        0x47        => Instr::Ld(Dst8::R8(RB), Src8::R8(RA)),
        0x48..=0x4d => Instr::Ld(Dst8::R8(RC), Src8::R8(r_src)),
        0x4e        => Instr::Ld(Dst8::R8(RC), Src8::Id(RHL)),
        0x4f        => Instr::Ld(Dst8::R8(RC), Src8::R8(RA)),

        0x50..=0x55 => Instr::Ld(Dst8::R8(RD), Src8::R8(r_src)),
        0x56        => Instr::Ld(Dst8::R8(RD), Src8::Id(RHL)),
        0x57        => Instr::Ld(Dst8::R8(RD), Src8::R8(RA)),
        0x58..=0x5d => Instr::Ld(Dst8::R8(RE), Src8::R8(r_src)),
        0x5e        => Instr::Ld(Dst8::R8(RE), Src8::Id(RHL)),
        0x5f        => Instr::Ld(Dst8::R8(RE), Src8::R8(RA)),

        0x60..=0x65 => Instr::Ld(Dst8::R8(RH), Src8::R8(r_src)),
        0x66        => Instr::Ld(Dst8::R8(RH), Src8::Id(RHL)),
        0x67        => Instr::Ld(Dst8::R8(RH), Src8::R8(RA)),
        0x68..=0x6d => Instr::Ld(Dst8::R8(RL), Src8::R8(r_src)),
        0x6e        => Instr::Ld(Dst8::R8(RL), Src8::Id(RHL)),
        0x6f        => Instr::Ld(Dst8::R8(RL), Src8::R8(RA)),

        0x70..=0x75 => Instr::Ld(Dst8::Id(RHL), Src8::R8(r_src)),
        0x76        => Instr::Halt,
        0x77        => Instr::Ld(Dst8::Id(RHL), Src8::R8(RA)),
        0x78..=0x7d => Instr::Ld(Dst8::R8(RA), Src8::R8(r_src)),
        0x7e        => Instr::Ld(Dst8::R8(RA), Src8::Id(RHL)),
        0x7f        => Instr::Ld(Dst8::R8(RA), Src8::R8(RA)),

        0x80..=0x85 => Instr::Add(Src8::R8(r_src)),
        0x86        => Instr::Add(Src8::Id(RHL)),
        0x87        => Instr::Add(Src8::R8(RA)),
        0x88..=0x8d => Instr::Adc(Src8::R8(r_src)),
        0x8e        => Instr::Adc(Src8::Id(RHL)),
        0x8f        => Instr::Adc(Src8::R8(RA)),

        0x90..=0x95 => Instr::Sub(Src8::R8(r_src)),
        0x96        => Instr::Sub(Src8::Id(RHL)),
        0x97        => Instr::Sub(Src8::R8(RA)),
        0x98..=0x9d => Instr::Sbc(Src8::R8(r_src)),
        0x9e        => Instr::Sbc(Src8::Id(RHL)),
        0x9f        => Instr::Sbc(Src8::R8(RA)),

        0xa0..=0xa5 => Instr::And(Src8::R8(r_src)),
        0xa6        => Instr::And(Src8::Id(RHL)),
        0xa7        => Instr::And(Src8::R8(RA)),
        0xa8..=0xad => Instr::Xor(Src8::R8(r_src)),
        0xae        => Instr::Xor(Src8::Id(RHL)),
        0xaf        => Instr::Xor(Src8::R8(RA)),

        0xb0..=0xb5 => Instr::Or(Src8::R8(r_src)),
        0xb6        => Instr::Or(Src8::Id(RHL)),
        0xb7        => Instr::Or(Src8::R8(RA)),
        0xb8..=0xbd => Instr::Cp(Src8::R8(r_src)),
        0xbe        => Instr::Cp(Src8::Id(RHL)),
        0xbf        => Instr::Cp(Src8::R8(RA)),

        0xc1        => Instr::Pop(RBC),
        0xc5        => Instr::Push(RBC),
        0xc6        => Instr::Add(Src8::D8(n)),
        0xce        => Instr::Adc(Src8::D8(n)),

        0xd1        => Instr::Pop(RDE),
        0xd3        => panic!("Invalid opcode {:#2x}", opcode),
        0xd5        => Instr::Push(RDE),
        0xd6        => Instr::Sub(Src8::D8(n)),
        0xdb        => panic!("Invalid opcode {:#2x}", opcode),
        0xdd        => panic!("Invalid opcode {:#2x}", opcode),
        0xde        => Instr::Sbc(Src8::D8(n)),

        0xe0        => Instr::Ld(Dst8::IdFF(n), Src8::R8(RA)),
        0xe1        => Instr::Pop(RHL),
        0xe2        => Instr::Ld(Dst8::IdFFRC, Src8::R8(RA)),
        0xe3        => panic!("Invalid opcode {:#2x}", opcode),
        0xe4        => panic!("Invalid opcode {:#2x}", opcode),
        0xe5        => Instr::Push(RHL),
        0xe6        => Instr::And(Src8::D8(n)),
        0xe8        => Instr::Add16SP(n as i8),
        0xea        => Instr::Ld(Dst8::IdNN(nn), Src8::R8(RA)),
        0xeb        => panic!("Invalid opcode {:#2x}", opcode),
        0xec        => panic!("Invalid opcode {:#2x}", opcode),
        0xed        => panic!("Invalid opcode {:#2x}", opcode),
        0xee        => Instr::Xor(Src8::D8(n)),

        0xf0        => Instr::Ld(Dst8::R8(RA), Src8::IdFF(n)),
        0xf1        => Instr::Pop(RAF),
        0xf2        => Instr::Ld(Dst8::R8(RA), Src8::IdFFRC),
        0xf4        => panic!("Invalid opcode {:#2x}", opcode),
        0xf5        => Instr::Push(RAF),
        0xf6        => Instr::Or(Src8::D8(n)),
        0xf8        => Instr::Ld16(Dst16::R16(RHL), Src16::SPD8(n as i8)),
        0xf9        => Instr::Ld16(Dst16::RSP, Src16::R16(RHL)),
        0xfa        => Instr::Ld(Dst8::R8(RA), Src8::IdNN(nn)),
        0xfc        => panic!("Invalid opcode {:#2x}", opcode),
        0xfd        => panic!("Invalid opcode {:#2x}", opcode),
        0xfe        => Instr::Cp(Src8::D8(n)),
        
        _ => panic!("TODO implement opcode {:#2x}", opcode)
    }
}