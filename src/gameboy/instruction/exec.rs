use std::num::Wrapping;
use super::super::gameboy::{*};
use super::instruction::{
    CarryMode, Src8, Dst8, Src16, Dst16, BitwiseOp, IncDec, AddSub, Cond
};

const BIT_0: u8 = 0b0000_0001;
const BIT_7: u8 = 0b1000_0000;

pub fn stop(gb: &mut Gameboy) {
    gb.stopped = true;
}

pub fn halt(gb: &mut Gameboy)  {
    gb.halted = true;
}

pub fn di(gb: &mut Gameboy) {
    gb.ime = false;
}

pub fn ei(gb: &mut Gameboy) {
    // TODO this should take effect after next machine cycle, apparently
    // Should probably schedule this by specifying by what cycle IME should
    // get enabled. For >2 cycle instructions, IME will be set by the time the
    // next start of the emulation loop. For 1 cycle instructions, IME will end up
    // being enabled a cycle too early (could matter).
    gb.ime = true;
}

pub fn ccf(gb: &mut Gameboy) {
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
    let c = (gb.regs[RF] & FLAG_C) >> 4;
    if c == 1 {
        gb.regs[RF] &= !FLAG_C;
    } else {
        gb.regs[RF] |= FLAG_C; 
    }
}

pub fn scf(gb: &mut Gameboy) {
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
    gb.regs[RF] |= FLAG_C;
}

pub fn daa(gb: &mut Gameboy) {
    // TODO implement
}

pub fn cpl(gb: &mut Gameboy) {
    gb.regs[RA] = !gb.regs[RA];
    gb.regs[RF] |= FLAG_N;
    gb.regs[RF] |= FLAG_H;
}

/// 8-bit load.
pub fn ld(gb: &mut Gameboy, dst: &Dst8, src: &Src8) {
    let value = Src8::get_value(gb, &src);
    Dst8::set_value(gb, &dst, value);
}

/// 8-bit load to/from (HL), but increment or decrement HL afterwards.
pub fn ld_inc_dec(gb: &mut Gameboy, dst: &Dst8, src: &Src8, mode: IncDec) {
    let value = Src8::get_value(gb, &src);
    Dst8::set_value(gb, &dst, value);

    let offset = match mode {
        IncDec::Inc => 1,
        IncDec::Dec => -1,
    };
    let new = (rr_to_u16(gb, RHL) as i16) + offset;
    gb.regs[RH] = (new >> 8) as u8;
    gb.regs[RL] = new as u8;
}

/// 16-bit load.
pub fn ld_16(gb: &mut Gameboy, dst: &Dst16, src: &Src16) {
    if let Src16::SPD8(n) = src {
        gb.regs[RF] &= 0;
        gb.regs[RF] |= compute_half_carry_flag(gb.sp as u8, *n as u8);
        gb.regs[RF] |= compute_carry_flag(gb.sp as u8, *n as u8);
    }

    let value = Src16::get_value(gb, &src);
    Dst16::set_value(gb, &dst, value);
}

/// Push to stack memory, data from 16-bit register.
pub fn push(gb: &mut Gameboy, r_pair: &RR) {
    gb.sp -= 1;
    gb.mem[gb.sp as usize] = gb.regs[r_pair.0];
    gb.sp -= 1;
    gb.mem[gb.sp as usize] = gb.regs[r_pair.1];
}

/// Pop to 16-bit register, data from stack memory.
pub fn pop(gb: &mut Gameboy, r_pair: &RR) {
    gb.regs[r_pair.1] = gb.mem[gb.sp as usize];
    gb.sp += 1;
    gb.regs[r_pair.0] = gb.mem[gb.sp as usize];
    gb.sp += 1;
}

/// 8-bit increment/decrement.
pub fn inc_dec(gb: &mut Gameboy, dst: &Dst8, mode: IncDec) {
    let value = Dst8::get_value(gb, &dst);
    let computed_value = match mode {
        IncDec::Inc => (Wrapping(value) + Wrapping(1)).0,
        IncDec::Dec => (Wrapping(value) + Wrapping((-1 as i8) as u8)).0,
    };

    gb.regs[RF] &= !(FLAG_Z ^ FLAG_N ^ FLAG_H);
    if let IncDec::Dec = mode { gb.regs[RF] |= FLAG_N; }
    gb.regs[RF] |= compute_zero_flag(computed_value);
    gb.regs[RF] |= compute_half_carry_flag(value, match mode {
        IncDec::Inc => 1,
        IncDec::Dec => (-1 as i8) as u8,
    });

    Dst8::set_value(gb, &dst, computed_value);
}

pub fn add_sub(gb: &mut Gameboy, src: &Src8, mode: AddSub, carry_mode: CarryMode) {
    let mut value = Src8::get_value(gb, &src);
    if let CarryMode::WithCarry = carry_mode {
        value += (gb.regs[RF] & FLAG_C) >> 4
    }
    let sum = match mode {
        AddSub::Add => (Wrapping(gb.regs[RA]) + Wrapping(value)).0,
        AddSub::Sub => (Wrapping(gb.regs[RA]) - Wrapping(value)).0,
    };

    gb.regs[RF] &= 0;
    gb.regs[RF] |= compute_zero_flag(sum);
    if let AddSub::Sub = mode { 
        gb.regs[RF] |= FLAG_N;
        gb.regs[RF] |= FLAG_H & !compute_half_carry_flag(gb.regs[RA], -(value as i8) as u8);
        gb.regs[RF] |= FLAG_C & !compute_carry_flag(gb.regs[RA], -(value as i8) as u8);
    } else {
        gb.regs[RF] |= compute_half_carry_flag(gb.regs[RA], value);
        gb.regs[RF] |= compute_carry_flag(gb.regs[RA], value);
    }
    
    gb.regs[RA] = sum;
}

pub fn bitwise(gb: &mut Gameboy, src: &Src8, operation: BitwiseOp) {
    let value = Src8::get_value(gb, &src);
    let computed_value = match operation {
        BitwiseOp::And => gb.regs[RA] & value,
        BitwiseOp::Xor => gb.regs[RA] ^ value,
        BitwiseOp::Or  => gb.regs[RA] | value,
    };

    gb.regs[RF] &= FLAG_H;
    gb.regs[RF] |= compute_zero_flag(computed_value);

    gb.regs[RA] = computed_value;
}

pub fn cp(gb: &mut Gameboy, src: &Src8) {
    let value = Src8::get_value(gb, &src);
    let sum = (Wrapping(gb.regs[RA]) - Wrapping(value)).0;

    gb.regs[RF] &= 0;
    gb.regs[RF] |= compute_zero_flag(sum);
    gb.regs[RF] |= FLAG_N;
    gb.regs[RF] |= FLAG_H & !compute_half_carry_flag(gb.regs[RA], -(value as i8) as u8);
    gb.regs[RF] |= FLAG_C & !compute_carry_flag(gb.regs[RA], -(value as i8) as u8);
}

pub fn add_16_hl(gb: &mut Gameboy, src: &Src16) {
    let value = Src16::get_value(gb, &src);
    let sum = (Wrapping(rr_to_u16(gb, RHL)) + Wrapping(value)).0;

    gb.regs[RF] &= !(FLAG_N ^ FLAG_H ^ FLAG_C);
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] |= compute_half_carry_flag(gb.regs[RH], (value >> 8) as u8);
    gb.regs[RF] |= compute_carry_flag(gb.regs[RH], (value >> 8) as u8);

    gb.regs[RH] = (sum >> 8) as u8;
    gb.regs[RL] = sum as u8;
}

pub fn inc_dec_16(gb: &mut Gameboy, dst: &Dst16, mode: IncDec) {
    let value = Dst16::get_value(gb, &dst);
    let computed_value = match mode {
        IncDec::Inc => (Wrapping(value) + Wrapping(1)).0,
        IncDec::Dec => (Wrapping(value) + Wrapping((-1 as i16) as u16)).0,
    };

    Dst16::set_value(gb, &dst, computed_value);
}

pub fn jp(gb: &mut Gameboy, src: &Src16) {
    gb.pc = Src16::get_value(gb, &src);
}

pub fn jp_cond(gb: &mut Gameboy, cond: &Cond, addr: u16) {
    if Cond::check(gb, cond) {
        gb.pc = addr;
    }
}

pub fn jr(gb: &mut Gameboy, offset: i8) {
    gb.pc = (Wrapping(gb.pc as i16) + Wrapping(offset as i16)).0 as u16;
}

pub fn jr_cond(gb: &mut Gameboy, cond: &Cond, offset: i8) {
    if Cond::check(gb, cond) {
        gb.pc = (Wrapping(gb.pc as i16) + Wrapping(offset as i16)).0 as u16;
    }
}

pub fn call(gb: &mut Gameboy, addr: u16) {
    push_pc(gb);
    gb.pc = addr;
}

pub fn call_cond(gb: &mut Gameboy, cond: &Cond, addr: u16) {
    if Cond::check(gb, cond) {
        push_pc(gb);
        gb.pc = addr;
    }
}

pub fn ret(gb: &mut Gameboy) {
    pop_pc(gb);
}

pub fn ret_cond(gb: &mut Gameboy, cond: &Cond) {
    if Cond::check(gb, cond) {
        pop_pc(gb);
    }
}

pub fn reti(gb: &mut Gameboy) {
    pop_pc(gb);
    gb.ime = true;
}

pub fn rst(gb: &mut Gameboy, addr: u8) {
    push_pc(gb);
    gb.pc = addr as u16;
}

pub fn rlca(gb: &mut Gameboy) {
    rlc(gb, &Dst8::R8(RA));
    gb.regs[RF] &= !FLAG_Z;
}

pub fn rla(gb: &mut Gameboy) {
    rl(gb, &Dst8::R8(RA));
    gb.regs[RF] &= !FLAG_Z;
}

pub fn rrca(gb: &mut Gameboy) {
    rrc(gb, &Dst8::R8(RA));
    gb.regs[RF] &= !FLAG_Z;
}

pub fn rra(gb: &mut Gameboy) {
    rr(gb, &Dst8::R8(RA));
    gb.regs[RF] &= !FLAG_Z;
}

pub fn rlc(gb: &mut Gameboy, dst: &Dst8) {
    // Copy bit 7 to both carry and bit 0
    let value = Dst8::get_value(gb, &dst);
    let bit7 = value >> 7;
    Dst8::set_value(gb, dst, value << 1);
    let value = Dst8::get_value(gb, &dst);
    if bit7 == 0 {
        gb.regs[RF] &= !FLAG_C;
        Dst8::set_value(gb, dst, value & !BIT_0);
    } else {
        gb.regs[RF] |= FLAG_C;
        Dst8::set_value(gb, dst, value | BIT_0);
    }
    gb.regs[RF] |= compute_zero_flag(Dst8::get_value(gb, &dst));
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
}

pub fn rrc(gb: &mut Gameboy, dst: &Dst8) {
    // Copy bit 0 to both carry and bit 7
    let value = Dst8::get_value(gb, &dst);
    let bit0 = value & BIT_0;
    Dst8::set_value(gb, dst, value >> 1);
    let value = Dst8::get_value(gb, &dst);
    if bit0 == 0 {
        gb.regs[RF] &= !FLAG_C;
        Dst8::set_value(gb, dst, value & !BIT_7);
    } else {
        gb.regs[RF] |= FLAG_C;
        gb.regs[RA] |= BIT_7;
        Dst8::set_value(gb, dst, value | BIT_7);
    }
    gb.regs[RF] |= compute_zero_flag(Dst8::get_value(gb, &dst));
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
}

pub fn rl(gb: &mut Gameboy, dst: &Dst8) {
    // Copy carry to bit 0, bit 7 to carry
    let value = Dst8::get_value(gb, &dst);
    let c = (gb.regs[RF] & FLAG_C) >> 4;
    let bit7 = value >> 7;
    Dst8::set_value(gb, dst, value << 1);
    let value = Dst8::get_value(gb, &dst);
    if c == 0 {
        Dst8::set_value(gb, dst, value & !BIT_0);
    } else {
        Dst8::set_value(gb, dst, value | BIT_0);
    }
    if bit7 == 0 {
        gb.regs[RF] &= !FLAG_C;
    } else {
        gb.regs[RF] |= FLAG_C;
    }
    gb.regs[RF] |= compute_zero_flag(Dst8::get_value(gb, &dst));
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
}

pub fn rr(gb: &mut Gameboy, dst: &Dst8) {
    // Copy carry to bit 7, bit 0 to carry
    let value = Dst8::get_value(gb, &dst);
    let c = (gb.regs[RF] & FLAG_C) >> 4;
    let bit0 = value & BIT_0;
    Dst8::set_value(gb, dst, value >> 1);
    let value = Dst8::get_value(gb, &dst);
    if c == 0 {
        Dst8::set_value(gb, dst, value & !BIT_7);
    } else {
        Dst8::set_value(gb, dst, value | BIT_7);
    }
    if bit0 == 0 {
        gb.regs[RF] &= !FLAG_C;
    } else {
        gb.regs[RF] |= FLAG_C;
    }
    gb.regs[RF] |= compute_zero_flag(Dst8::get_value(gb, &dst));
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
}

pub fn sla(gb: &mut Gameboy, dst: &Dst8) {
    let value = Dst8::get_value(gb, &dst);
    let bit7 = value >> 7;
    Dst8::set_value(gb, dst, value << 1);
    if bit7 == 0 {
        gb.regs[RF] &= !FLAG_C;
    } else {
        gb.regs[RF] |= FLAG_C;
    }
    gb.regs[RF] |= compute_zero_flag(Dst8::get_value(gb, &dst));
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
}

pub fn sra(gb: &mut Gameboy, dst: &Dst8) {
    // Like a normal right shift, but bit 7 is repeated
    let value = Dst8::get_value(gb, &dst);
    let bit7 = value >> 7;
    let bit0 = value & BIT_0;
    Dst8::set_value(gb, dst, value >> 1);
    let value = Dst8::get_value(gb, &dst);
    if bit7 == 0 {
        Dst8::set_value(gb, &dst, value & !BIT_7);
    } else {
        Dst8::set_value(gb, &dst, value | BIT_7);
    }
    if bit0 == 0 {
        gb.regs[RF] &= !FLAG_C;
    } else {
        gb.regs[RF] |= FLAG_C;
    }
    gb.regs[RF] |= compute_zero_flag(Dst8::get_value(gb, &dst));
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
}

pub fn srl(gb: &mut Gameboy, dst: &Dst8) {
    let value = Dst8::get_value(gb, &dst);
    let bit0 = value & BIT_0;
    Dst8::set_value(gb, dst, value >> 1);
    if bit0 == 0 {
        gb.regs[RF] &= !FLAG_C;
    } else {
        gb.regs[RF] |= FLAG_C;
    }
    gb.regs[RF] |= compute_zero_flag(Dst8::get_value(gb, &dst));
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
}

pub fn swap(gb: &mut Gameboy, dst: &Dst8) {
    let value = Dst8::get_value(gb, &dst);
    let top = value >> 4;
    let bottom = value << 4;
    Dst8::set_value(gb, &dst, bottom | top);
}

pub fn bit(gb: &mut Gameboy, bt: u8, dst: &Dst8) {
    let value = Dst8::get_value(gb, &dst);
    if (value >> bt) == 0 {
        gb.regs[RF] |= FLAG_Z;
    } else {
        gb.regs[RF] &= !FLAG_Z;
    }
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] |= FLAG_H;
}

pub fn res(gb: &mut Gameboy, bt: u8, dst: &Dst8) {
    let value = Dst8::get_value(gb, &dst);
    Dst8::set_value(gb, &dst, value & !(BIT_0 << bt));
}

pub fn set(gb: &mut Gameboy, bt: u8, dst: &Dst8) {
    let value = Dst8::get_value(gb, &dst);
    Dst8::set_value(gb, &dst, value | (BIT_0 << bt));
}

// Helper functions

fn compute_zero_flag(x: u8) -> u8 {
    if x == 0 {0b1000_0000} else {0}
}

fn compute_half_carry_flag(x: u8, y: u8) -> u8 {
    // Compute a 4-bit sum, check if the 5th bit is 1. 
    // If so, then a half-carry occurred.
    let sum_4_bit = (x & 0b0000_1111) + (y & 0b0000_1111);
    (sum_4_bit & 0b0001_0000) << 1
}

fn compute_carry_flag(x: u8, y: u8) -> u8 {
    // Compute an 8-bit sum, check if the 9th bit is 1. 
    // If so, then a carry occurred.
    let sum_8_bit = (x as u16) + (y as u16);
    ((sum_8_bit & 0x100) >> 4) as u8
}

fn push_pc(gb: &mut Gameboy) {
    gb.sp -= 1;
    gb.mem[gb.sp as usize] = (gb.pc >> 8) as u8;
    gb.sp -= 1;
    gb.mem[gb.sp as usize] = gb.pc as u8;
}

fn pop_pc(gb: &mut Gameboy) {
    let lsb = gb.mem[gb.sp as usize];
    gb.sp += 1;
    let msb = gb.mem[gb.sp as usize];
    gb.sp += 1;
    gb.pc = ((msb as u16) << 8) | (lsb as u16);
}