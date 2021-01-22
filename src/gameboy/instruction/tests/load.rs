#[cfg(test)]
use std::sync::atomic::{Ordering};
use crate::gameboy::{*};
use super::super::step::step;
use super::utils::load_instr;

#[test]
fn ld_r8_r8() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x41));
    gb.regs[RB] = 0x12;
    gb.regs[RC] = 0x34;

    step(&mut gb);

    assert_eq!(gb.regs[RB], 0x34);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ld_r8_d8() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x06, 0x99));
    gb.regs[RB] = 0x12;

    step(&mut gb);

    assert_eq!(gb.regs[RB], 0x99);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0102);
}

#[test]
fn ld_hl_d8() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x36, 0x99));
    gb.mem[0x1234] = 0x05;
    gb.regs[RH] = 0x12;
    gb.regs[RL] = 0x34;

    step(&mut gb);

    assert_eq!(gb.mem[0x1234], 0x99);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0102);
}

#[test]
fn ld_id_r8() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x02));
    gb.mem[0x1234] = 0x05;
    gb.regs[RB] = 0x12;
    gb.regs[RC] = 0x34;
    gb.regs[RA] = 0x99;

    step(&mut gb);
    
    assert_eq!(gb.mem[0x1234], 0x99);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ld_r8_id() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x0a));
    gb.mem[0x1234] = 0x99;
    gb.regs[RB] = 0x12;
    gb.regs[RC] = 0x34;
    gb.regs[RA] = 0x05;

    step(&mut gb);
    
    assert_eq!(gb.regs[RA], 0x99);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ld_ra_hl_inc() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x2a));
    gb.mem[0x1234] = 0x99;
    gb.regs[RH] = 0x12;
    gb.regs[RL] = 0x34;
    gb.regs[RA] = 0x05;

    step(&mut gb);
    
    assert_eq!(gb.regs[RA], 0x99);
    assert_eq!(rr_to_u16(&mut gb, RHL), 0x1235);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ld_ra_hl_dec() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x3a));
    gb.mem[0x0000] = 0x99;
    gb.regs[RH] = 0x00;
    gb.regs[RL] = 0x00;
    gb.regs[RA] = 0x05;

    step(&mut gb);
    
    assert_eq!(gb.regs[RA], 0x99);
    assert_eq!(rr_to_u16(&mut gb, RHL), 0xffff);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ld_hl_ra_inc() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x22));
    gb.mem[0x1234] = 0x05;
    gb.regs[RH] = 0x12;
    gb.regs[RL] = 0x34;
    gb.regs[RA] = 0x99;

    step(&mut gb);

    assert_eq!(gb.mem[0x1234], 0x99);
    assert_eq!(rr_to_u16(&mut gb, RHL), 0x1235);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ld_hl_ra_dec() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x32));
    gb.mem[0x1234] = 0x05;
    gb.regs[RH] = 0x12;
    gb.regs[RL] = 0x34;
    gb.regs[RA] = 0x99;

    step(&mut gb);

    assert_eq!(gb.mem[0x1234], 0x99);
    assert_eq!(rr_to_u16(&mut gb, RHL), 0x1233);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ld_ra_nn() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xfa, 0x34, 0x12));
    gb.mem[0x1234] = 0x99;
    gb.regs[RA] = 0x05;

    step(&mut gb);

    assert_eq!(gb.regs[RA], 0x99);
    assert_eq!(gb.cycles, 4);
    assert_eq!(gb.pc, 0x0103);
}

#[test]
fn ld_nn_ra() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xea, 0x34, 0x12));
    gb.mem[0x1234] = 0x05;
    gb.regs[RA] = 0x99;

    step(&mut gb);

    assert_eq!(gb.regs[RA], 0x99);
    assert_eq!(gb.cycles, 4);
    assert_eq!(gb.pc, 0x0103);
}

#[test]
fn ldh_ra_rc() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xf2));
    gb.io_regs[0x03].store(0x99, Ordering::Relaxed);
    gb.regs[RC] = 0x03;
    gb.regs[RA] = 0x05;

    step(&mut gb);

    assert_eq!(gb.regs[RA], 0x99);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ldh_rc_ra() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xe2));
    gb.io_regs[0x03].store(0x05, Ordering::Relaxed);
    gb.regs[RC] = 0x03;
    gb.regs[RA] = 0x99;

    step(&mut gb);

    assert_eq!(gb.io_regs[0x03].load(Ordering::Relaxed), 0x99);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ldh_ra_n()  {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xf0, 0x03));
    gb.io_regs[0x03].store(0x99, Ordering::Relaxed);
    gb.regs[RA] = 0x05;

    step(&mut gb);

    assert_eq!(gb.regs[RA], 0x99);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0102);
}

#[test]
fn ldh_n_ra()  {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xe0, 0x03));
    gb.io_regs[0x03].store(0x05, Ordering::Relaxed);
    gb.regs[RA] = 0x99;

    step(&mut gb);

    assert_eq!(gb.io_regs[0x03].load(Ordering::Relaxed), 0x99);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0102);
}

#[test]
fn ld_r16_d16() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x11, 0xef, 0xbe));
    gb.regs[RD] = 0x05;
    gb.regs[RE] = 0x06;

    step(&mut gb);

    assert_eq!(rr_to_u16(&gb, RDE), 0xbeef);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0103);
}

#[test]
fn ld_rsp_d16() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x31, 0xef, 0xbe));
    gb.sp = 0x0506;

    step(&mut gb);

    assert_eq!(gb.sp, 0xbeef);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0103);
}

#[test]
fn ld_nn_sp() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x08, 0x34, 0x12));
    gb.mem[0x1234] = 0x05;
    gb.mem[0x1235] = 0x06;
    gb.sp = 0xbeef;

    step(&mut gb);

    assert_eq!(gb.mem[0x1234], 0xbe);
    assert_eq!(gb.mem[0x1235], 0xef);
    assert_eq!(gb.cycles, 5);
    assert_eq!(gb.pc, 0x0103);
}

#[test]
fn ld_sp_hl() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xf9));
    gb.regs[RH] = 0xbe;
    gb.regs[RL] = 0xef;
    gb.sp = 0x0506;

    step(&mut gb);

    assert_eq!(gb.sp, 0xbeef);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn push() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xc5));
    gb.regs[RB] = 0xbe;
    gb.regs[RC] = 0xef;
    gb.sp = 0x0600;

    step(&mut gb);

    assert_eq!(gb.sp, 0x05fe);
    assert_eq!(gb.mem[0x05ff], 0xbe);
    assert_eq!(gb.mem[0x05fe], 0xef);
    assert_eq!(gb.cycles, 4);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn pop() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xc1));
    gb.regs[RB] = 0x05;
    gb.regs[RC] = 0x06;
    gb.sp = 0x05fe;
    gb.mem[0x05ff] = 0xbe;
    gb.mem[0x05fe] = 0xef;

    step(&mut gb);

    assert_eq!(gb.sp, 0x0600);
    assert_eq!(gb.regs[RB], 0xbe);
    assert_eq!(gb.regs[RC], 0xef);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ld_hl_sp_r8_positive() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xf8, 0x07));
    gb.regs[RH] = 0x05;
    gb.regs[RL] = 0x06;
    gb.sp = 0x0600;

    step(&mut gb);

    assert_eq!(gb.regs[RH], 0x06);
    assert_eq!(gb.regs[RL], 0x07);
    assert_eq!(gb.regs[RF], 0);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0102);
}

#[test]
fn ld_hl_sp_r8_negative() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xf8, (-7 as i8) as u8));
    gb.regs[RH] = 0x05;
    gb.regs[RL] = 0x06;
    gb.sp = 0x0600;

    step(&mut gb);

    assert_eq!(gb.regs[RH], 0x05);
    assert_eq!(gb.regs[RL], 0xf9);
    assert_eq!(gb.regs[RF], 0);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0102);
}

#[test]
fn ld_hl_sp_r8_flag_c() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xf8, (-7 as i8) as u8));
    gb.regs[RH] = 0x05;
    gb.regs[RL] = 0x06;
    gb.sp = 0x0680;

    step(&mut gb);

    assert_eq!(gb.regs[RH], 0x06);
    assert_eq!(gb.regs[RL], 0x79);
    assert_eq!(gb.regs[RF], FLAG_C);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0102);
}

#[test]
fn ld_hl_sp_r8_flag_h() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xf8, 0x79));
    gb.regs[RH] = 0x05;
    gb.regs[RL] = 0x06;
    gb.sp = 0x0608;

    step(&mut gb);

    assert_eq!(gb.regs[RH], 0x06);
    assert_eq!(gb.regs[RL], 0x81);
    assert_eq!(gb.regs[RF], FLAG_H);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0102);
}

#[test]
fn ld_hl_sp_r8_flag_hc() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xf8, (-7 as i8) as u8));
    gb.regs[RH] = 0x05;
    gb.regs[RL] = 0x06;
    gb.sp = 0x0688;

    step(&mut gb);

    assert_eq!(gb.regs[RH], 0x06);
    assert_eq!(gb.regs[RL], 0x81);
    assert_eq!(gb.regs[RF], FLAG_H | FLAG_C);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0102);
}