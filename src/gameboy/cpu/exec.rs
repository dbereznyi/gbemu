use std::num::Wrapping;
use std::sync::atomic::{Ordering};
use super::super::gameboy::{*};
use super::instruction::{
    CarryMode, Src8, Dst8, Src16, Dst16, BitwiseOp, IncDec, AddSub, Cond
};

const BIT_0: u8 = 0b0000_0001;
const BIT_7: u8 = 0b1000_0000;

pub fn stop(gb: &mut Gameboy) {
    gb.stopped.store(true, Ordering::Relaxed);
}

pub fn halt(gb: &mut Gameboy)  {
    if gb.ime.load(Ordering::Relaxed) {
        gb.halted.store(true, Ordering::Relaxed);
    }
    // TODO handle instruction-skipping behavior
}

pub fn di(gb: &mut Gameboy) {
    gb.ime.store(false, Ordering::Relaxed);
}

pub fn ei(gb: &mut Gameboy) {
    // TODO this should take effect after next machine cycle, apparently
    // Should probably schedule this by specifying by what cycle IME should
    // get enabled. For >2 cycle instructions, IME will be set by the time the
    // next start of the emulation loop. For 1 cycle instructions, IME will end up
    // being enabled a cycle too early (could matter).
    gb.ime.store(true, Ordering::Relaxed);
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
    let n = gb.regs[RF] & FLAG_N > 0;
    let c = gb.regs[RF] & FLAG_C > 0;
    let h = gb.regs[RF] & FLAG_H > 0;
    let a_low = gb.regs[RA] & 0b0000_1111;
    let a_high = (gb.regs[RA] & 0b1111_0000) >> 4;

    // TODO a real GB probably wouldn't lock up here, but not sure right now what it would actually
    // do
    let (add_to_a, new_c) = match (n, c, a_high, h, a_low) {
        (false, false, high, false, low) => {
            if (0x00..=0x09).contains(&high) && (0x00..=0x09).contains(&low) {
                (0x00, 0)
            } else if (0x00..=0x08).contains(&high) && (0x0a..=0x0f).contains(&low) {
                (0x06, 0)
            } else if (0x0a..=0x0f).contains(&high) && (0x00..=0x09).contains(&low) {
                (0x60, 1)
            } else if (0x09..=0x0f).contains(&high) && (0x0a..=0x0f).contains(&low) {
                (0x66, 1)
            } else {
                panic!("Invalid register A value 0x{:0>2X} for N={},C={},H={}",
                       gb.regs[RA], n, c, h)
            }
        },
        (false, false, high, true, low) => {
            if (0x00..=0x09).contains(&high) && (0x00..=0x03).contains(&low) {
                (0x06, 0)
            } else if (0x0a..=0x0f).contains(&high) && (0x00..=0x03).contains(&low) {
                (0x66, 1)
            } else {
                panic!("Invalid register A value 0x{:0>2X} for N={},C={},H={}",
                       gb.regs[RA], n, c, h)
            }
        },
        (false, true, high, false, low) => {
            if (0x00..=0x02).contains(&high) && (0x00..=0x09).contains(&low) {
                (0x60, 1)
            } else if (0x00..=0x02).contains(&high) && (0x0a..=0x0f).contains(&low) {
                (0x66, 1)
            } else {
                panic!("Invalid register A value 0x{:0>2X} for N={},C={},H={}",
                       gb.regs[RA], n, c, h)
            }
        },
        (false, true, high, true, low) => {
            if (0x00..=0x03).contains(&high) && (0x00..=0x03).contains(&low) {
                (0x66, 1)
            } else {
                panic!("Invalid register A value 0x{:0>2X} for N={},C={},H={}",
                       gb.regs[RA], n, c, h)
            }
        },
        (true, false, high, false, low) => {
            if (0x00..=0x09).contains(&high) && (0x00..=0x09).contains(&low) {
                (0x00, 0)
            } else {
                panic!("Invalid register A value 0x{:0>2X} for N={},C={},H={}",
                       gb.regs[RA], n, c, h)
            }
        },
        (true, false, high, true, low) => {
            if (0x00..=0x08).contains(&high) && (0x06..=0x0f).contains(&low) {
                (0xfa, 0)
            } else {
                panic!("Invalid register A value 0x{:0>2X} for N={},C={},H={}",
                       gb.regs[RA], n, c, h)
            }
        },
        (true, true, high, false, low) => {
            if (0x07..=0x0f).contains(&high) && (0x00..=0x09).contains(&low) {
                (0xa0, 1)
            } else {
                panic!("Invalid register A value 0x{:0>2X} for N={},C={},H={}",
                       gb.regs[RA], n, c, h)
            }
        },
        (true, true, high, true, low) => {
            if (0x06..=0x0f).contains(&high) && (0x06..=0x0f).contains(&low) {
                (0x9a, 0)
            } else {
                panic!("Invalid register A value 0x{:0>2X} for N={},C={},H={}",
                       gb.regs[RA], n, c, h)
            }
        },
    };

    gb.regs[RA] = (Wrapping(gb.regs[RA]) + Wrapping(add_to_a)).0;

    if gb.regs[RA] == 0 {
        gb.regs[RF] |= FLAG_Z;
    } else {
        gb.regs[RF] &= !FLAG_Z;
    }

    gb.regs[RF] &= !FLAG_H;

    if new_c == 1 {
        gb.regs[RF] |= FLAG_C;
    } else {
        gb.regs[RF] &= !FLAG_C;
    }
}

pub fn cpl(gb: &mut Gameboy) {
    gb.regs[RA] = !gb.regs[RA];
    gb.regs[RF] |= FLAG_N;
    gb.regs[RF] |= FLAG_H;
}

pub fn ld(gb: &mut Gameboy, dst: Dst8, src: Src8) {
    let value = src.read(gb);
    dst.write(gb, value);
}

pub fn ld_inc_dec(gb: &mut Gameboy, dst: Dst8, src: Src8, mode: IncDec) {
    let value = src.read(gb);
    dst.write(gb, value);

    let offset = match mode {
        IncDec::Inc => 1,
        IncDec::Dec => -1,
    };
    let new = (rr_to_u16(gb, RHL) as i16) + offset;
    gb.regs[RH] = (new >> 8) as u8;
    gb.regs[RL] = new as u8;
}

pub fn ld_16(gb: &mut Gameboy, dst: Dst16, src: Src16) {
    if let Src16::SPD8(n) = src {
        gb.regs[RF] &= 0;
        gb.regs[RF] |= compute_half_carry_flag(gb.sp as u8, n as u8);
        gb.regs[RF] |= compute_carry_flag(gb.sp as u8, n as u8);
    }

    let value = src.read(gb);
    dst.write(gb, value);
}

pub fn push(gb: &mut Gameboy, r_pair: RR) {
    gb.sp -= 1;
    gb.write(gb.sp, gb.regs[r_pair.0]);
    gb.sp -= 1;
    gb.write(gb.sp, gb.regs[r_pair.1]);
}

pub fn pop(gb: &mut Gameboy, r_pair: RR) {
    gb.regs[r_pair.1] = gb.read(gb.sp);
    gb.sp += 1;
    gb.regs[r_pair.0] = gb.read(gb.sp);
    gb.sp += 1;
}

pub fn inc_dec(gb: &mut Gameboy, dst: Dst8, mode: IncDec) {
    let value = dst.read(gb);
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

    dst.write(gb, computed_value);
}

pub fn add_sub(gb: &mut Gameboy, src: Src8, mode: AddSub, carry_mode: CarryMode) {
    let mut value = src.read(gb);
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

pub fn bitwise(gb: &mut Gameboy, src: Src8, operation: BitwiseOp) {
    let value = src.read(gb);
    let computed_value = match operation {
        BitwiseOp::And => gb.regs[RA] & value,
        BitwiseOp::Xor => gb.regs[RA] ^ value,
        BitwiseOp::Or  => gb.regs[RA] | value,
    };

    gb.regs[RF] &= FLAG_H;
    gb.regs[RF] |= compute_zero_flag(computed_value);

    gb.regs[RA] = computed_value;
}

pub fn cp(gb: &mut Gameboy, src: Src8) {
    let value = src.read(gb);
    let sum = (Wrapping(gb.regs[RA]) - Wrapping(value)).0;

    gb.regs[RF] &= 0;
    gb.regs[RF] |= compute_zero_flag(sum);
    gb.regs[RF] |= FLAG_N;
    gb.regs[RF] |= FLAG_H & !compute_half_carry_flag(gb.regs[RA], -(value as i8) as u8);
    gb.regs[RF] |= FLAG_C & !compute_carry_flag(gb.regs[RA], -(value as i8) as u8);
}

pub fn add_16_hl(gb: &mut Gameboy, src: Src16) {
    let value = src.read(gb);
    let sum = (Wrapping(rr_to_u16(gb, RHL)) + Wrapping(value)).0;

    gb.regs[RF] &= !(FLAG_N ^ FLAG_H ^ FLAG_C);
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] |= compute_half_carry_flag(gb.regs[RH], (value >> 8) as u8);
    gb.regs[RF] |= compute_carry_flag(gb.regs[RH], (value >> 8) as u8);

    gb.regs[RH] = (sum >> 8) as u8;
    gb.regs[RL] = sum as u8;
}

pub fn add_16_sp(gb: &mut Gameboy, n: i8) {
    let value = Src16::SPD8(n).read(gb);
    Dst16::RSP.write(gb, value);
}

pub fn inc_dec_16(gb: &mut Gameboy, dst: Dst16, mode: IncDec) {
    let value = dst.read(gb);
    let new_value = match mode {
        IncDec::Inc => (Wrapping(value) + Wrapping(1)).0,
        IncDec::Dec => (Wrapping(value) + Wrapping((-1 as i16) as u16)).0,
    };

    dst.write(gb, new_value);
}

pub fn jp(gb: &mut Gameboy, src: Src16) {
    gb.pc = src.read(gb);
}

pub fn jp_cond(gb: &mut Gameboy, cond: Cond, addr: u16) {
    if cond.check(gb) {
        gb.pc = addr;
    }
}

pub fn jr(gb: &mut Gameboy, offset: i8) {
    gb.pc = (Wrapping(gb.pc as i16) + Wrapping(2 as i16) + Wrapping(offset as i16)).0 as u16;
}

pub fn jr_cond(gb: &mut Gameboy, cond: Cond, offset: i8) {
    if cond.check(gb) {
        gb.pc = (Wrapping(gb.pc as i16) + Wrapping(2 as i16) + Wrapping(offset as i16)).0 as u16;
    }
}

pub fn call(gb: &mut Gameboy, addr: u16) {
    gb.pc += 3;
    push_pc(gb);
    gb.pc = addr;
}

pub fn call_cond(gb: &mut Gameboy, cond: Cond, addr: u16) {
    if cond.check(gb) {
        gb.pc += 3;
        push_pc(gb);
        gb.pc = addr;
    }
}

pub fn ret(gb: &mut Gameboy) {
    pop_pc(gb);
}

pub fn ret_cond(gb: &mut Gameboy, cond: Cond) {
    if cond.check(gb) {
        pop_pc(gb);
    }
}

pub fn reti(gb: &mut Gameboy) {
    pop_pc(gb);
    gb.ime.store(true, Ordering::Relaxed);
}

pub fn rst(gb: &mut Gameboy, addr: u8) {
    push_pc(gb);
    gb.pc = addr as u16;
}

pub fn rlca(gb: &mut Gameboy) {
    rlc(gb, Dst8::R8(RA));
    gb.regs[RF] &= !FLAG_Z;
}

pub fn rla(gb: &mut Gameboy) {
    rl(gb, Dst8::R8(RA));
    gb.regs[RF] &= !FLAG_Z;
}

pub fn rrca(gb: &mut Gameboy) {
    rrc(gb, Dst8::R8(RA));
    gb.regs[RF] &= !FLAG_Z;
}

pub fn rra(gb: &mut Gameboy) {
    rr(gb, Dst8::R8(RA));
    gb.regs[RF] &= !FLAG_Z;
}

pub fn rlc(gb: &mut Gameboy, dst: Dst8) {
    // Copy bit 7 to both carry and bit 0
    let value = dst.read(gb);
    let bit7 = value >> 7;
    dst.write(gb, value << 1);
    let value = dst.read(gb);
    if bit7 == 0 {
        gb.regs[RF] &= !FLAG_C;
        dst.write(gb, value & !BIT_0);
    } else {
        gb.regs[RF] |= FLAG_C;
        dst.write(gb, value | BIT_0);
    }
    gb.regs[RF] |= compute_zero_flag(dst.read(gb));
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
}

pub fn rrc(gb: &mut Gameboy, dst: Dst8) {
    // Copy bit 0 to both carry and bit 7
    let value = dst.read(gb);
    let bit0 = value & BIT_0;
    dst.write(gb, value >> 1);
    let value = dst.read(gb);
    if bit0 == 0 {
        gb.regs[RF] &= !FLAG_C;
        dst.write(gb, value & !BIT_7);
    } else {
        gb.regs[RF] |= FLAG_C;
        gb.regs[RA] |= BIT_7;
        dst.write(gb, value | BIT_7);
    }
    gb.regs[RF] |= compute_zero_flag(dst.read(gb));
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
}

pub fn rl(gb: &mut Gameboy, dst: Dst8) {
    // Copy carry to bit 0, bit 7 to carry
    let value = dst.read(gb);
    let c = (gb.regs[RF] & FLAG_C) >> 4;
    let bit7 = value >> 7;
    dst.write(gb, value << 1);
    let value = dst.read(gb);
    if c == 0 {
        dst.write(gb, value & !BIT_0);
    } else {
        dst.write(gb, value | BIT_0);
    }
    if bit7 == 0 {
        gb.regs[RF] &= !FLAG_C;
    } else {
        gb.regs[RF] |= FLAG_C;
    }
    gb.regs[RF] |= compute_zero_flag(dst.read(gb));
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
}

pub fn rr(gb: &mut Gameboy, dst: Dst8) {
    // Copy carry to bit 7, bit 0 to carry
    let value = dst.read(gb);
    let c = (gb.regs[RF] & FLAG_C) >> 4;
    let bit0 = value & BIT_0;
    dst.write(gb, value >> 1);
    let value = dst.read(gb);
    if c == 0 {
        dst.write(gb, value & !BIT_7);
    } else {
        dst.write(gb, value | BIT_7);
    }
    if bit0 == 0 {
        gb.regs[RF] &= !FLAG_C;
    } else {
        gb.regs[RF] |= FLAG_C;
    }
    gb.regs[RF] |= compute_zero_flag(dst.read(gb));
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
}

pub fn sla(gb: &mut Gameboy, dst: Dst8) {
    let value = dst.read(gb);
    let bit7 = value >> 7;
    dst.write(gb, value << 1);
    if bit7 == 0 {
        gb.regs[RF] &= !FLAG_C;
    } else {
        gb.regs[RF] |= FLAG_C;
    }
    gb.regs[RF] |= compute_zero_flag(dst.read(gb));
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
}

pub fn sra(gb: &mut Gameboy, dst: Dst8) {
    // Like a normal right shift, but bit 7 is repeated
    let value = dst.read(gb);
    let bit7 = value >> 7;
    let bit0 = value & BIT_0;
    dst.write(gb, value >> 1);
    let value = dst.read(gb);
    if bit7 == 0 {
        dst.write(gb, value & !BIT_7);
    } else {
        dst.write(gb, value | BIT_7);
    }
    if bit0 == 0 {
        gb.regs[RF] &= !FLAG_C;
    } else {
        gb.regs[RF] |= FLAG_C;
    }
    gb.regs[RF] |= compute_zero_flag(dst.read(gb));
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
}

pub fn srl(gb: &mut Gameboy, dst: Dst8) {
    let value = dst.read(gb);
    let bit0 = value & BIT_0;
    dst.write(gb, value >> 1);
    if bit0 == 0 {
        gb.regs[RF] &= !FLAG_C;
    } else {
        gb.regs[RF] |= FLAG_C;
    }
    gb.regs[RF] |= compute_zero_flag(dst.read(gb));
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] &= !FLAG_H;
}

pub fn swap(gb: &mut Gameboy, dst: Dst8) {
    let value = dst.read(gb);
    let top = value >> 4;
    let bottom = value << 4;
    dst.write(gb, bottom | top);
}

pub fn bit(gb: &mut Gameboy, bt: u8, dst: Dst8) {
    let value = dst.read(gb);
    if (value & (1 << bt)) == 0 {
        gb.regs[RF] |= FLAG_Z;
    } else {
        gb.regs[RF] &= !FLAG_Z;
    }
    gb.regs[RF] &= !FLAG_N;
    gb.regs[RF] |= FLAG_H;
}

pub fn res(gb: &mut Gameboy, bt: u8, dst: Dst8) {
    let value = dst.read(gb);
    dst.write(gb, value & !(BIT_0 << bt));
}

pub fn set(gb: &mut Gameboy, bt: u8, dst: Dst8) {
    let value = dst.read(gb);
    dst.write(gb, value | (BIT_0 << bt));
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

pub fn push_pc(gb: &mut Gameboy) {
    gb.sp -= 1;
    gb.write(gb.sp, (gb.pc >> 8) as u8);
    gb.sp -= 1;
    gb.write(gb.sp, gb.pc as u8);
}

fn pop_pc(gb: &mut Gameboy) {
    let lsb = gb.read(gb.sp);
    gb.sp += 1;
    let msb = gb.read(gb.sp);
    gb.sp += 1;
    gb.pc = ((msb as u16) << 8) | (lsb as u16);
}
