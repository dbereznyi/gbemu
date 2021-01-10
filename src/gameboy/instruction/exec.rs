use std::num::Wrapping;
use super::super::gameboy::{*};
use super::instruction::{CarryMode, Src8, Dst8, Src16, Dst16, BitwiseOp, IncDec, AddSub};

pub fn stop(_gb: &mut Gameboy) {
    // TODO implement
}

pub fn halt(_gb: &mut Gameboy) {
    // TODO implement
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
    offset_r16(gb, RHL, offset);
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

// Helper functions

/// Offset (i.e add or subtract) a 16-bit register by a given amount
fn offset_r16(gb: &mut Gameboy, r16: RR, offset: i16) {
    let new = (rr_to_u16(gb, r16) as i16) + offset;
    gb.regs[r16.0] = (new >> 8) as u8;
    gb.regs[r16.1] = new as u8;
}

fn compute_zero_flag(x: u8) -> u8 {
    if x == 0 {0b10000000} else {0}
}

fn compute_half_carry_flag(x: u8, y: u8) -> u8 {
    // Compute a 4-bit sum, check if the 5th bit is 1. 
    // If so, then a half-carry occurred.
    let sum_4_bit = (x & 0b00001111) + (y & 0b00001111);
    (sum_4_bit & 0b00010000) << 1
}

fn compute_carry_flag(x: u8, y: u8) -> u8 {
    // Compute an 8-bit sum, check if the 9th bit is 1. 
    // If so, then a carry occurred.
    let sum_8_bit = (x as u16) + (y as u16);
    ((sum_8_bit & 0x100) >> 4) as u8
}