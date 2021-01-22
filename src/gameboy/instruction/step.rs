use super::super::gameboy::{*};
use super::exec::{*};
use super::instruction::{
    CarryMode, Src8, Dst8, Src16, Dst16, BitwiseOp, IncDec, AddSub, Cond, Instr
};

/// Decode the instruction at PC, then execute it and update PC and cycle count accordingly.
pub fn step(gb: &mut Gameboy) {
    let instr = decode(gb);
    match &instr {
        // Control/misc
        Instr::Nop             => (),
        Instr::Stop            => stop(gb),
        Instr::Halt            => halt(gb),
        Instr::Di              => di(gb),
        Instr::Ei              => ei(gb),
        Instr::Ccf             => ccf(gb),
        Instr::Scf             => scf(gb),
        Instr::Daa             => daa(gb),
        Instr::Cpl             => cpl(gb),
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
        // Control-flow
        Instr::Jp(src)         => jp(gb, src),
        Instr::JpCC(cc, nn)    => jp_cond(gb, cc, *nn),
        Instr::Jr(offset)      => jr(gb, *offset),
        Instr::JrCC(cc, off)   => jr_cond(gb, cc, *off),
        Instr::Call(nn)        => call(gb, *nn),
        Instr::CallCC(cc, nn)  => call_cond(gb, cc, *nn),
        Instr::Ret             => ret(gb),
        Instr::RetCC(cc)       => ret_cond(gb, cc),
        Instr::Reti            => reti(gb),
        Instr::Rst(n)          => rst(gb, *n),
        // Rotates, shifts, bit operations
        Instr::Rlca            => rlca(gb),
        Instr::Rla             => rla(gb),
        Instr::Rrca            => rrca(gb),
        Instr::Rra             => rra(gb),
        Instr::Rlc(dst)        => rlc(gb, dst),
        Instr::Rrc(dst)        => rrc(gb, dst),
        Instr::Rl(dst)         => rl(gb, dst),
        Instr::Rr(dst)         => rr(gb, dst),
        Instr::Sla(dst)        => sla(gb, dst),
        Instr::Sra(dst)        => sra(gb, dst),
        Instr::Srl(dst)        => srl(gb, dst),
        Instr::Bit(bt, dst)    => bit(gb, *bt, dst),
        Instr::Res(bt, dst)    => res(gb, *bt, dst),
        Instr::Set(bt, dst)    => set(gb, *bt, dst),
        Instr::Swap(dst)       => swap(gb, dst),
    }
    gb.pc += Instr::size(&instr);
    gb.cycles += Instr::num_cycles(gb, &instr);
}

/// Decode the current instruction.
fn decode(gb: &Gameboy) -> Instr {
    let opcode = gb.mem[gb.pc as usize];
    // The bottom three bits of the opcode are used to indicate src reg for certain loads
    //let r_src = (opcode & 0b0000_0111) as usize;
    let src_reg = reg_encoding_to_src(opcode & 0b0000_0111);
    // Grab data from PC+1 and PC+2 in case we need them as arguments
    // This shouldn't go out of bounds since instructions aren't executed in top of mem
    let n = gb.mem[(gb.pc + 1) as usize];
    let n2 = gb.mem[(gb.pc + 2) as usize];
    // Convert from little-endian, n is lsb and n2 is msb
    let nn = ((n2 as u16) << 8) | (n as u16);
    // For 0xCB instructions, n encodes a register in the bottom three bits
    let cb_reg = reg_encoding_to_dst(n & 0b0000_0111);
    match opcode {
        0x00        => Instr::Nop,
        0x01        => Instr::Ld16(Dst16::R16(RBC), Src16::D16(nn)),
        0x02        => Instr::Ld(Dst8::Id(RBC), Src8::R8(RA)),
        0x03        => Instr::Inc16(Dst16::R16(RBC)),
        0x04        => Instr::Inc(Dst8::R8(RB)),
        0x05        => Instr::Dec(Dst8::R8(RB)),
        0x06        => Instr::Ld(Dst8::R8(RB), Src8::D8(n)),
        0x07        => Instr::Rlca,
        0x08        => Instr::Ld16(Dst16::IdNN(nn), Src16::RSP),
        0x09        => Instr::Add16HL(Src16::R16(RBC)),
        0x0a        => Instr::Ld(Dst8::R8(RA), Src8::Id(RBC)),
        0x0b        => Instr::Dec16(Dst16::R16(RBC)),
        0x0c        => Instr::Inc(Dst8::R8(RC)),
        0x0d        => Instr::Dec(Dst8::R8(RC)),
        0x0e        => Instr::Ld(Dst8::R8(RC), Src8::D8(n)),
        0x0f        => Instr::Rrca,

        0x10        => Instr::Stop,
        0x11        => Instr::Ld16(Dst16::R16(RDE), Src16::D16(nn)),
        0x12        => Instr::Ld(Dst8::Id(RDE), Src8::R8(RA)),
        0x13        => Instr::Inc16(Dst16::R16(RDE)),
        0x14        => Instr::Inc(Dst8::R8(RD)),
        0x15        => Instr::Dec(Dst8::R8(RD)),
        0x16        => Instr::Ld(Dst8::R8(RD), Src8::D8(n)),
        0x17        => Instr::Rla,
        0x18        => Instr::Jr(n as i8),
        0x19        => Instr::Add16HL(Src16::R16(RDE)),
        0x1a        => Instr::Ld(Dst8::R8(RA), Src8::Id(RDE)),
        0x1b        => Instr::Dec16(Dst16::R16(RDE)),
        0x1c        => Instr::Inc(Dst8::R8(RE)),
        0x1d        => Instr::Dec(Dst8::R8(RE)),
        0x1e        => Instr::Ld(Dst8::R8(RE), Src8::D8(n)),
        0x1f        => Instr::Rra,

        0x20        => Instr::JrCC(Cond::Nz, n as i8),
        0x21        => Instr::Ld16(Dst16::R16(RHL), Src16::D16(nn)),
        0x22        => Instr::LdInc(Dst8::Id(RHL), Src8::R8(RA)),
        0x23        => Instr::Inc16(Dst16::R16(RHL)),
        0x24        => Instr::Inc(Dst8::R8(RH)),
        0x25        => Instr::Dec(Dst8::R8(RH)),
        0x26        => Instr::Ld(Dst8::R8(RH), Src8::D8(n)),
        0x27        => Instr::Daa,
        0x28        => Instr::JrCC(Cond::Z, n as i8),
        0x29        => Instr::Add16HL(Src16::R16(RHL)),
        0x2a        => Instr::LdInc(Dst8::R8(RA), Src8::Id(RHL)),
        0x2b        => Instr::Inc16(Dst16::R16(RHL)),
        0x2c        => Instr::Inc(Dst8::R8(RL)),
        0x2d        => Instr::Dec(Dst8::R8(RL)),
        0x2e        => Instr::Ld(Dst8::R8(RL), Src8::D8(n)),
        0x2f        => Instr::Cpl,

        0x30        => Instr::JrCC(Cond::Nc, n as i8),
        0x31        => Instr::Ld16(Dst16::RSP, Src16::D16(nn)),
        0x32        => Instr::LdDec(Dst8::Id(RHL), Src8::R8(RA)),
        0x33        => Instr::Inc16(Dst16::RSP),
        0x34        => Instr::Inc(Dst8::Id(RHL)),
        0x35        => Instr::Dec(Dst8::Id(RHL)),
        0x36        => Instr::Ld(Dst8::Id(RHL), Src8::D8(n)),
        0x37        => Instr::Scf,
        0x38        => Instr::JrCC(Cond::C, n as i8),
        0x39        => Instr::Add16HL(Src16::RSP),
        0x3a        => Instr::LdDec(Dst8::R8(RA), Src8::Id(RHL)),
        0x3b        => Instr::Dec16(Dst16::RSP),
        0x3c        => Instr::Inc(Dst8::R8(RA)),
        0x3d        => Instr::Dec(Dst8::R8(RA)),
        0x3e        => Instr::Ld(Dst8::R8(RA), Src8::D8(n)),
        0x3f        => Instr::Ccf,

        0x40..=0x47 => Instr::Ld(Dst8::R8(RB), src_reg),
        0x48..=0x4f => Instr::Ld(Dst8::R8(RC), src_reg),
        0x50..=0x57 => Instr::Ld(Dst8::R8(RD), src_reg),
        0x58..=0x5f => Instr::Ld(Dst8::R8(RE), src_reg),
        0x60..=0x67 => Instr::Ld(Dst8::R8(RH), src_reg),
        0x68..=0x6f => Instr::Ld(Dst8::R8(RL), src_reg),
        0x70..=0x75 => Instr::Ld(Dst8::Id(RHL), src_reg),
        0x76        => Instr::Halt,
        0x77        => Instr::Ld(Dst8::Id(RHL), Src8::R8(RA)),
        0x78..=0x7f => Instr::Ld(Dst8::R8(RA), src_reg),

        0x80..=0x87 => Instr::Add(src_reg),
        0x88..=0x8f => Instr::Adc(src_reg),
        0x90..=0x97 => Instr::Sub(src_reg),
        0x98..=0x9f => Instr::Sbc(src_reg),
        0xa0..=0xa7 => Instr::And(src_reg),
        0xa8..=0xaf => Instr::Xor(src_reg),
        0xb0..=0xb7 => Instr::Or(src_reg),
        0xb8..=0xbf => Instr::Cp(src_reg),

        0xc0        => Instr::RetCC(Cond::Nz),
        0xc1        => Instr::Pop(RBC),
        0xc2        => Instr::JpCC(Cond::Nz, nn),
        0xc3        => Instr::Jp(Src16::D16(nn)),
        0xc4        => Instr::CallCC(Cond::Nz, nn),
        0xc5        => Instr::Push(RBC),
        0xc6        => Instr::Add(Src8::D8(n)),
        0xc7        => Instr::Rst(0x00),
        0xc8        => Instr::RetCC(Cond::Z),
        0xc9        => Instr::Ret,
        0xca        => Instr::JpCC(Cond::Z, nn),
        0xcb        => match n {
            0x00..=0x07 => Instr::Rlc(cb_reg),
            0x08..=0x0f => Instr::Rrc(cb_reg),
            0x10..=0x17 => Instr::Rl(cb_reg),
            0x18..=0x1f => Instr::Rr(cb_reg),
            0x20..=0x27 => Instr::Sla(cb_reg),
            0x28..=0x2f => Instr::Sra(cb_reg),
            0x30..=0x37 => Instr::Swap(cb_reg),
            0x38..=0x3f => Instr::Srl(cb_reg),

            0x40..=0x47 => Instr::Bit(0, cb_reg),
            0x48..=0x4f => Instr::Bit(1, cb_reg),
            0x50..=0x57 => Instr::Bit(2, cb_reg),
            0x58..=0x5f => Instr::Bit(3, cb_reg),
            0x60..=0x67 => Instr::Bit(4, cb_reg),
            0x68..=0x6f => Instr::Bit(5, cb_reg),
            0x70..=0x77 => Instr::Bit(6, cb_reg),
            0x78..=0x7f => Instr::Bit(7, cb_reg),

            0x80..=0x87 => Instr::Res(0, cb_reg),
            0x88..=0x8f => Instr::Res(1, cb_reg),
            0x90..=0x97 => Instr::Res(2, cb_reg),
            0x98..=0x9f => Instr::Res(3, cb_reg),
            0xa0..=0xa7 => Instr::Res(4, cb_reg),
            0xa8..=0xaf => Instr::Res(5, cb_reg),
            0xb0..=0xb7 => Instr::Res(6, cb_reg),
            0xb8..=0xbf => Instr::Res(7, cb_reg),

            0xc0..=0xc7 => Instr::Set(0, cb_reg),
            0xc8..=0xcf => Instr::Set(1, cb_reg),
            0xd0..=0xd7 => Instr::Set(2, cb_reg),
            0xd8..=0xdf => Instr::Set(3, cb_reg),
            0xe0..=0xe7 => Instr::Set(4, cb_reg),
            0xe8..=0xef => Instr::Set(5, cb_reg),
            0xf0..=0xf7 => Instr::Set(6, cb_reg),
            0xf8..=0xff => Instr::Set(7, cb_reg),
        },
        0xcc        => Instr::CallCC(Cond::Z, nn),
        0xcd        => Instr::Call(nn),
        0xce        => Instr::Adc(Src8::D8(n)),
        0xcf        => Instr::Rst(0x08),

        0xd0        => Instr::RetCC(Cond::Nc),
        0xd1        => Instr::Pop(RDE),
        0xd2        => Instr::JpCC(Cond::Nc, nn),
        0xd3        => panic!("Invalid opcode {:#2x}", opcode),
        0xd4        => Instr::CallCC(Cond::Nc, nn),
        0xd5        => Instr::Push(RDE),
        0xd6        => Instr::Sub(Src8::D8(n)),
        0xd7        => Instr::Rst(0x10),
        0xd8        => Instr::RetCC(Cond::C),
        0xd9        => Instr::Reti,
        0xda        => Instr::JpCC(Cond::C, nn),
        0xdb        => panic!("Invalid opcode {:#2x}", opcode),
        0xdc        => Instr::CallCC(Cond::C, nn),
        0xdd        => panic!("Invalid opcode {:#2x}", opcode),
        0xde        => Instr::Sbc(Src8::D8(n)),
        0xdf        => Instr::Rst(0x18),

        0xe0        => Instr::Ld(Dst8::IdFF(n), Src8::R8(RA)),
        0xe1        => Instr::Pop(RHL),
        0xe2        => Instr::Ld(Dst8::IdFFRC, Src8::R8(RA)),
        0xe3        => panic!("Invalid opcode {:#2x}", opcode),
        0xe4        => panic!("Invalid opcode {:#2x}", opcode),
        0xe5        => Instr::Push(RHL),
        0xe6        => Instr::And(Src8::D8(n)),
        0xe7        => Instr::Rst(0x20),
        0xe8        => Instr::Add16SP(n as i8),
        0xe9        => Instr::Jp(Src16::R16(RHL)),
        0xea        => Instr::Ld(Dst8::IdNN(nn), Src8::R8(RA)),
        0xeb        => panic!("Invalid opcode {:#2x}", opcode),
        0xec        => panic!("Invalid opcode {:#2x}", opcode),
        0xed        => panic!("Invalid opcode {:#2x}", opcode),
        0xee        => Instr::Xor(Src8::D8(n)),
        0xef        => Instr::Rst(0x28),

        0xf0        => Instr::Ld(Dst8::R8(RA), Src8::IdFF(n)),
        0xf1        => Instr::Pop(RAF),
        0xf2        => Instr::Ld(Dst8::R8(RA), Src8::IdFFRC),
        0xf3        => Instr::Di,
        0xf4        => panic!("Invalid opcode {:#2x}", opcode),
        0xf5        => Instr::Push(RAF),
        0xf6        => Instr::Or(Src8::D8(n)),
        0xf7        => Instr::Rst(0x30),
        0xf8        => Instr::Ld16(Dst16::R16(RHL), Src16::SPD8(n as i8)),
        0xf9        => Instr::Ld16(Dst16::RSP, Src16::R16(RHL)),
        0xfa        => Instr::Ld(Dst8::R8(RA), Src8::IdNN(nn)),
        0xfb        => Instr::Ei,
        0xfc        => panic!("Invalid opcode {:#2x}", opcode),
        0xfd        => panic!("Invalid opcode {:#2x}", opcode),
        0xfe        => Instr::Cp(Src8::D8(n)),
        0xff        => Instr::Rst(0x38),
    }
}

/// Converts an opcode register encoding to the corresponding Dst8 value.
fn reg_encoding_to_dst(encoding: u8) -> Dst8 {
    match encoding {
        0 => Dst8::R8(RB),
        1 => Dst8::R8(RC),
        2 => Dst8::R8(RD),
        3 => Dst8::R8(RE),
        4 => Dst8::R8(RH),
        5 => Dst8::R8(RL),
        6 => Dst8::Id(RHL),
        7 => Dst8::R8(RA),
        _ => panic!("Invalid encoding: {:0>3b}", encoding),
    }
}

// Repetitive, but okay for now I guess
fn reg_encoding_to_src(encoding: u8) -> Src8 {
    match encoding {
        0 => Src8::R8(RB),
        1 => Src8::R8(RC),
        2 => Src8::R8(RD),
        3 => Src8::R8(RE),
        4 => Src8::R8(RH),
        5 => Src8::R8(RL),
        6 => Src8::Id(RHL),
        7 => Src8::R8(RA),
        _ => panic!("Invalid encoding: {:0>3b}", encoding),
    }
}