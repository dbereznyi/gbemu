#[cfg(test)]
use crate::gameboy::{*};
use super::super::step;
use super::utils::load_instr;

#[test]
fn inc_r8() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x0c));
    gb.regs[RC] = 0x05;

    step(&mut gb);

    assert_eq!(gb.regs[RC], 0x06);
    assert_eq!(gb.regs[RF], 0);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn inc_r8_overflow() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x0c));
    gb.regs[RC] = 0xff;

    step(&mut gb);

    assert_eq!(gb.regs[RC], 0x00);
    assert_eq!(gb.regs[RF], FLAG_Z | FLAG_H);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn dec_r8() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x0d));
    gb.regs[RC] = 0x05;

    step(&mut gb);

    assert_eq!(gb.regs[RC], 0x04);
    assert_eq!(gb.regs[RF], FLAG_N | FLAG_H);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn dec_r8_z() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x0d));
    gb.regs[RC] = 0x01;

    step(&mut gb);

    assert_eq!(gb.regs[RC], 0x00);
    assert_eq!(gb.regs[RF], FLAG_Z | FLAG_N | FLAG_H);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn dec_r8_underflow() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x0d));
    gb.regs[RC] = 0x00;

    step(&mut gb);

    assert_eq!(gb.regs[RC], 0xff);
    assert_eq!(gb.regs[RF], FLAG_N);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn inc_id_hl() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x34));
    gb.mem[0x1234] = 0x05;
    gb.regs[RH] = 0x12;
    gb.regs[RL] = 0x34;

    step(&mut gb);

    assert_eq!(gb.mem[0x1234], 0x06);
    assert_eq!(gb.regs[RF], 0);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn inc_id_hl_overflow() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x34));
    gb.mem[0x1234] = 0xff;
    gb.regs[RH] = 0x12;
    gb.regs[RL] = 0x34;

    step(&mut gb);

    assert_eq!(gb.mem[0x1234], 0x00);
    assert_eq!(gb.regs[RF], FLAG_Z | FLAG_H);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn dec_id_hl() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x35));
    gb.mem[0x1234] = 0x05;
    gb.regs[RH] = 0x12;
    gb.regs[RL] = 0x34;

    step(&mut gb);

    assert_eq!(gb.mem[0x1234], 0x04);
    assert_eq!(gb.regs[RF], FLAG_N | FLAG_H);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn dec_id_hl_z() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x35));
    gb.mem[0x1234] = 0x01;
    gb.regs[RH] = 0x12;
    gb.regs[RL] = 0x34;

    step(&mut gb);

    assert_eq!(gb.mem[0x1234], 0x00);
    assert_eq!(gb.regs[RF], FLAG_Z | FLAG_N | FLAG_H);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn dec_id_hl_underflow() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x35));
    gb.mem[0x1234] = 0x00;
    gb.regs[RH] = 0x12;
    gb.regs[RL] = 0x34;

    step(&mut gb);

    assert_eq!(gb.mem[0x1234], 0xFF);
    assert_eq!(gb.regs[RF], FLAG_N);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn add_r8() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x83));
    gb.regs[RF] = FLAG_C;
    gb.regs[RA] = 0x05;
    gb.regs[RE] = 0x94;

    step(&mut gb);

    assert_eq!(gb.regs[RA], 0x99);
    assert_eq!(gb.regs[RF], 0);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn adc_r8() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x8b));
    gb.regs[RF] = FLAG_C;
    gb.regs[RA] = 0x05;
    gb.regs[RE] = 0x94;

    step(&mut gb);

    assert_eq!(gb.regs[RA], 0x9A);
    assert_eq!(gb.regs[RF], 0);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn sub_r8() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x93));
    gb.regs[RF] = FLAG_C;
    gb.regs[RA] = 0x99;
    gb.regs[RE] = 0x05;

    step(&mut gb);

    assert_eq!(gb.regs[RA], 0x94);
    assert_eq!(gb.regs[RF], FLAG_N);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn sbc_r8() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x9b));
    gb.regs[RF] = FLAG_C;
    gb.regs[RA] = 0x99;
    gb.regs[RE] = 0x05;

    step(&mut gb);

    assert_eq!(gb.regs[RA], 0x93);
    assert_eq!(gb.regs[RF], FLAG_N);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn cp_eq() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xbb));
    gb.regs[RA] = 0x99;
    gb.regs[RE] = 0x99;

    step(&mut gb);

    assert_eq!(gb.regs[RF], FLAG_Z | FLAG_N);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn cp_lt() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xbb));
    gb.regs[RA] = 0x99;
    gb.regs[RE] = 0x9a;

    step(&mut gb);

    assert_eq!(gb.regs[RF], FLAG_N | FLAG_H | FLAG_C);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn cp_gt() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0xbb));
    gb.regs[RA] = 0x9a;
    gb.regs[RE] = 0x99;

    step(&mut gb);

    assert_eq!(gb.regs[RF], FLAG_N);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn add_16_hl() {
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x19));
    gb.regs[RH] = 0x05;
    gb.regs[RL] = 0x06;
    gb.regs[RD] = 0x00;
    gb.regs[RE] = 0xff;
    gb.regs[RF] = FLAG_Z | FLAG_N;

    step(&mut gb);

    assert_eq!(gb.regs[RF], FLAG_Z);
    assert_eq!(gb.regs[RH], 0x06);
    assert_eq!(gb.regs[RL], 0x05);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn add_16_hl_h() {
    // TODO finish
    let mut gb = Gameboy::new();
    load_instr(&mut gb, vec!(0x19));
    gb.regs[RH] = 0x05;
    gb.regs[RL] = 0x06;
    gb.regs[RD] = 0x00;
    gb.regs[RE] = 0xff;
    gb.regs[RF] = FLAG_Z | FLAG_N;

    step(&mut gb);

    assert_eq!(gb.regs[RF], FLAG_Z);
    assert_eq!(gb.regs[RH], 0x06);
    assert_eq!(gb.regs[RL], 0x05);
    assert_eq!(gb.cycles, 2);
    assert_eq!(gb.pc, 0x0101);
}