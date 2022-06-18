#[cfg(test)]
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
    gb.write(0xc234, 0x05);
    gb.regs[RH] = 0xc2;
    gb.regs[RL] = 0x34;

    step(&mut gb);

    assert_eq!(gb.read(0xc234), 0x99);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0102);
}

#[test]
fn ld_id_r8() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x02));
    gb.write(0xc234, 0x05);
    gb.regs[RB] = 0xc2;
    gb.regs[RC] = 0x34;
    gb.regs[RA] = 0x99;

    step(&mut gb);
    
    assert_eq!(gb.read(0xc234), 0x99);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ld_r8_id() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x0a));
    gb.write(0xc234, 0x99);
    gb.regs[RB] = 0xc2;
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
    gb.write(0xc234, 0x99);
    gb.regs[RH] = 0xc2;
    gb.regs[RL] = 0x34;
    gb.regs[RA] = 0x05;

    step(&mut gb);
    
    assert_eq!(gb.regs[RA], 0x99);
    assert_eq!(rr_to_u16(&mut gb, RHL), 0xc235);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ld_ra_hl_dec() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x3a));
    gb.write(0xc234, 0x99);
    gb.regs[RH] = 0xc2;
    gb.regs[RL] = 0x34;
    gb.regs[RA] = 0x05;

    step(&mut gb);
    
    assert_eq!(gb.regs[RA], 0x99);
    assert_eq!(rr_to_u16(&mut gb, RHL), 0xc233);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ld_hl_ra_inc() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x22));
    gb.write(0xc234, 0x05);
    gb.regs[RH] = 0xc2;
    gb.regs[RL] = 0x34;
    gb.regs[RA] = 0x99;

    step(&mut gb);

    assert_eq!(gb.read(0xc234), 0x99);
    assert_eq!(rr_to_u16(&mut gb, RHL), 0xc235);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ld_hl_ra_dec() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x32));
    gb.write(0xc234, 0x05);
    gb.regs[RH] = 0xc2;
    gb.regs[RL] = 0x34;
    gb.regs[RA] = 0x99;

    step(&mut gb);

    assert_eq!(gb.read(0xc234), 0x99);
    assert_eq!(rr_to_u16(&mut gb, RHL), 0xc233);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ld_ra_nn() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xfa, 0x34, 0xc2));
    gb.write(0xc234, 0x99);
    gb.regs[RA] = 0x05;

    step(&mut gb);

    assert_eq!(gb.regs[RA], 0x99);
    assert_eq!(gb.cycles, 4);
    assert_eq!(gb.pc, 0x0103);
}

#[test]
fn ld_nn_ra() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xea, 0x34, 0xc2));
    gb.write(0xc234, 0x05);
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
    gb.write(0xff00 + 0x03, 0x99);
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
    gb.write(0xff00 + 0x03, 0x05);
    gb.regs[RC] = 0x03;
    gb.regs[RA] = 0x99;

    step(&mut gb);

    assert_eq!(gb.read(0xff00 + 0x03), 0x99);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn ldh_ra_n()  {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xf0, 0x03));
    gb.write(0xff00 + 0x03, 0x99);
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
    gb.write(0xff00 + 0x03, 0x05);
    gb.regs[RA] = 0x99;

    step(&mut gb);

    assert_eq!(gb.read(0xff00 + 0x03), 0x99);
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
    load_instr(&mut gb, vec!(0x08, 0x34, 0xc2));
    gb.write(0xc234, 0x05);
    gb.write(0xc235, 0x06);
    gb.sp = 0xfffe;

    step(&mut gb);

    assert_eq!(gb.read(0xc234), 0xff);
    assert_eq!(gb.read(0xc235), 0xfe);
    assert_eq!(gb.cycles, 5);
    assert_eq!(gb.pc, 0x0103);
}

#[test]
fn ld_sp_hl() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xf9));
    gb.regs[RH] = 0xff;
    gb.regs[RL] = 0xee;
    gb.sp = 0xfffe;

    step(&mut gb);

    assert_eq!(gb.sp, 0xffee);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn push() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xc5));
    gb.regs[RB] = 0xbe;
    gb.regs[RC] = 0xef;
    gb.sp = 0xfffe;

    step(&mut gb);

    assert_eq!(gb.sp, 0xfffc);
    assert_eq!(gb.read(0xfffd), 0xbe);
    assert_eq!(gb.read(0xfffc), 0xef);
    assert_eq!(gb.cycles, 4);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn pop() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xc1));
    gb.regs[RB] = 0x05;
    gb.regs[RC] = 0x06;
    gb.sp = 0xfffc;
    gb.write(0xfffd, 0xbe);
    gb.write(0xfffc, 0xef);

    step(&mut gb);

    assert_eq!(gb.sp, 0xfffe);
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
    gb.sp = 0xcf00;

    step(&mut gb);

    assert_eq!(gb.regs[RH], 0xcf);
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
    gb.sp = 0xcf00;

    step(&mut gb);

    assert_eq!(gb.regs[RH], 0xce);
    assert_eq!(gb.regs[RL], 0xf9);
    assert_eq!(gb.regs[RF], 0);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0102);
}

#[test]
fn ld_hl_sp_r8_flag_c() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xf8, 0x80));
    gb.regs[RH] = 0x05;
    gb.regs[RL] = 0x06;
    gb.sp = 0xfffd;

    step(&mut gb);

    assert_eq!(gb.regs[RH], 0xff);
    assert_eq!(gb.regs[RL], 0x7d);
    assert_eq!(gb.regs[RF], FLAG_C);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0102);
}

#[test]
fn ld_hl_sp_r8_flag_h() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xf8, 0x08));
    gb.regs[RH] = 0x05;
    gb.regs[RL] = 0x06;
    gb.sp = 0xff88;

    step(&mut gb);

    assert_eq!(gb.regs[RH], 0xff);
    assert_eq!(gb.regs[RL], 0x90);
    assert_eq!(gb.regs[RF], FLAG_H);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0102);
}

#[test]
fn ld_hl_sp_r8_flag_hc() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xf8, 0x88));
    gb.regs[RH] = 0x05;
    gb.regs[RL] = 0x06;
    gb.sp = 0xc08c;

    step(&mut gb);

    assert_eq!(gb.regs[RH], 0xc0);
    assert_eq!(gb.regs[RL], 0x14);
    assert_eq!(gb.regs[RF], FLAG_H | FLAG_C);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0102);
}
