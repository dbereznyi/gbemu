#[cfg(test)]
use crate::gameboy::{*};
use super::super::step;
use super::utils::test_cartridge;

#[test]
fn inc_r8() {
    let cartridge = test_cartridge(vec!(0x0c));
    let mut gb = Gameboy::new(cartridge);
    gb.regs[RC] = 0x05;

    step(&mut gb);

    assert_eq!(gb.regs[RC], 0x06);
    assert_eq!(gb.regs[RF], 0);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn inc_r8_overflow() {
    let cartridge = test_cartridge(vec!(0x0c));
    let mut gb = Gameboy::new(cartridge);
    gb.regs[RC] = 0xff;

    step(&mut gb);

    assert_eq!(gb.regs[RC], 0x00);
    assert_eq!(gb.regs[RF], FLAG_Z | FLAG_H);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn dec_r8() {
    let cartridge = test_cartridge(vec!(0x0d));
    let mut gb = Gameboy::new(cartridge);
    gb.regs[RC] = 0x05;

    step(&mut gb);

    assert_eq!(gb.regs[RC], 0x04);
    assert_eq!(gb.regs[RF], FLAG_N | FLAG_H);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn dec_r8_z() {
    let cartridge = test_cartridge(vec!(0x0d));
    let mut gb = Gameboy::new(cartridge);
    gb.regs[RC] = 0x01;

    step(&mut gb);

    assert_eq!(gb.regs[RC], 0x00);
    assert_eq!(gb.regs[RF], FLAG_Z | FLAG_N | FLAG_H);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn dec_r8_underflow() {
    let cartridge = test_cartridge(vec!(0x0d));
    let mut gb = Gameboy::new(cartridge);
    gb.regs[RC] = 0x00;

    step(&mut gb);

    assert_eq!(gb.regs[RC], 0xff);
    assert_eq!(gb.regs[RF], FLAG_N);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn inc_id_hl() {
    let cartridge = test_cartridge(vec!(0x34));
    let mut gb = Gameboy::new(cartridge);
    gb.write(0xc234, 0x05);
    gb.regs[RH] = 0xc2;
    gb.regs[RL] = 0x34;

    step(&mut gb);

    assert_eq!(gb.read(0xc234), 0x06);
    assert_eq!(gb.regs[RF], 0);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn inc_id_hl_overflow() {
    let cartridge = test_cartridge(vec!(0x34));
    let mut gb = Gameboy::new(cartridge);
    gb.write(0xc234, 0xff);
    gb.regs[RH] = 0xc2;
    gb.regs[RL] = 0x34;

    step(&mut gb);

    assert_eq!(gb.read(0xc234), 0x00);
    assert_eq!(gb.regs[RF], FLAG_Z | FLAG_H);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn dec_id_hl() {
    let cartridge = test_cartridge(vec!(0x35));
    let mut gb = Gameboy::new(cartridge);
    gb.write(0xc234, 0x05);
    gb.regs[RH] = 0xc2;
    gb.regs[RL] = 0x34;

    step(&mut gb);

    assert_eq!(gb.read(0xc234), 0x04);
    assert_eq!(gb.regs[RF], FLAG_N | FLAG_H);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn dec_id_hl_z() {
    let cartridge = test_cartridge(vec!(0x35));
    let mut gb = Gameboy::new(cartridge);
    gb.write(0xc234, 0x01);
    gb.regs[RH] = 0xc2;
    gb.regs[RL] = 0x34;

    step(&mut gb);

    assert_eq!(gb.read(0xc234), 0x00);
    assert_eq!(gb.regs[RF], FLAG_Z | FLAG_N | FLAG_H);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn dec_id_hl_underflow() {
    let cartridge = test_cartridge(vec!(0x35));
    let mut gb = Gameboy::new(cartridge);
    gb.write(0xc234, 0x00);
    gb.regs[RH] = 0xc2;
    gb.regs[RL] = 0x34;

    step(&mut gb);

    assert_eq!(gb.read(0xc234), 0xFF);
    assert_eq!(gb.regs[RF], FLAG_N);
    assert_eq!(gb.cycles, 3);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn add_r8() {
    let cartridge = test_cartridge(vec!(0x83));
    let mut gb = Gameboy::new(cartridge);
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
    let cartridge = test_cartridge(vec!(0x8b));
    let mut gb = Gameboy::new(cartridge);
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
    let cartridge = test_cartridge(vec!(0x93));
    let mut gb = Gameboy::new(cartridge);
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
    let cartridge = test_cartridge(vec!(0x9b));
    let mut gb = Gameboy::new(cartridge);
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
    let cartridge = test_cartridge(vec!(0xbb));
    let mut gb = Gameboy::new(cartridge);
    gb.regs[RA] = 0x99;
    gb.regs[RE] = 0x99;

    step(&mut gb);

    assert_eq!(gb.regs[RF], FLAG_Z | FLAG_N);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn cp_lt() {
    let cartridge = test_cartridge(vec!(0xbb));
    let mut gb = Gameboy::new(cartridge);
    gb.regs[RA] = 0x99;
    gb.regs[RE] = 0x9a;

    step(&mut gb);

    assert_eq!(gb.regs[RF], FLAG_N | FLAG_H | FLAG_C);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn cp_gt() {
    let cartridge = test_cartridge(vec!(0xbb));
    let mut gb = Gameboy::new(cartridge);
    gb.regs[RA] = 0x9a;
    gb.regs[RE] = 0x99;

    step(&mut gb);

    assert_eq!(gb.regs[RF], FLAG_N);
    assert_eq!(gb.cycles, 1);
    assert_eq!(gb.pc, 0x0101);
}

#[test]
fn add_16_hl() {
    let cartridge = test_cartridge(vec!(0x19));
    let mut gb = Gameboy::new(cartridge);
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
    let cartridge = test_cartridge(vec!(0x19));
    let mut gb = Gameboy::new(cartridge);
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
