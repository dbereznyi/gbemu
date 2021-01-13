#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU8, Ordering};
    use crate::gameboy::{*};
    use super::super::exec;
    use super::super::instruction::{CarryMode, Src8, Dst8, Src16, Dst16, IncDec, AddSub};

    #[test]
    fn ld_r8_r8() {
        let mut gb = Gameboy::new();
        gb.regs[RB] = 0x12;
        gb.regs[RC] = 0x34;

        exec::ld(&mut gb, &Dst8::R8(RB), &Src8::R8(RC));

        assert_eq!(gb.regs[RB], 0x34);
    }

    #[test]
    fn ld_r8_d8() {
        let mut gb = Gameboy::new();
        gb.regs[RB] = 0x12;

        exec::ld(&mut gb, &Dst8::R8(RB), &Src8::D8(0x99));

        assert_eq!(gb.regs[RB], 0x99);
    }

    #[test]
    fn ld_hl_d8() {
        let mut gb = Gameboy::new();
        gb.mem[0x1234] = 0x05;
        gb.regs[RH] = 0x12;
        gb.regs[RL] = 0x34;

        exec::ld(&mut gb, &Dst8::Id(RHL), &Src8::D8(0x99));

        assert_eq!(gb.mem[0x1234], 0x99);
    }

    #[test]
    fn ld_id_r8() {
        let mut gb = Gameboy::new();
        gb.mem[0x1234] = 0x05;
        gb.regs[RB] = 0x12;
        gb.regs[RC] = 0x34;
        gb.regs[RA] = 0x99;

        exec::ld(&mut gb, &Dst8::Id(RBC), &Src8::R8(RA));
        
        assert_eq!(gb.mem[0x1234], 0x99);
    }

    #[test]
    fn ld_r8_id() {
        let mut gb = Gameboy::new();
        gb.mem[0x1234] = 0x99;
        gb.regs[RB] = 0x12;
        gb.regs[RC] = 0x34;
        gb.regs[RA] = 0x05;

        exec::ld(&mut gb, &Dst8::R8(RA), &Src8::Id(RBC));
        
        assert_eq!(gb.regs[RA], 0x99);
    }

    #[test]
    fn ld_ra_hl_inc() {
        let mut gb = Gameboy::new();
        gb.mem[0x1234] = 0x99;
        gb.regs[RH] = 0x12;
        gb.regs[RL] = 0x34;
        gb.regs[RA] = 0x05;

        exec::ld_inc_dec(&mut gb, &Dst8::R8(RA), &Src8::Id(RHL), IncDec::Inc);
        
        assert_eq!(gb.regs[RA], 0x99);
        assert_eq!(rr_to_u16(&mut gb, RHL), 0x1235);
    }

    #[test]
    fn ld_ra_hl_dec() {
        let mut gb = Gameboy::new();
        gb.mem[0x0000] = 0x99;
        gb.regs[RH] = 0x00;
        gb.regs[RL] = 0x00;
        gb.regs[RA] = 0x05;

        exec::ld_inc_dec(&mut gb, &Dst8::R8(RA), &Src8::Id(RHL), IncDec::Dec);
        
        assert_eq!(gb.regs[RA], 0x99);
        assert_eq!(rr_to_u16(&mut gb, RHL), 0xffff);
    }

    #[test]
    fn ld_hl_ra_inc() {
        let mut gb = Gameboy::new();
        gb.mem[0x1234] = 0x05;
        gb.regs[RH] = 0x12;
        gb.regs[RL] = 0x34;
        gb.regs[RA] = 0x99;

        exec::ld_inc_dec(&mut gb, &Dst8::Id(RHL), &Src8::R8(RA), IncDec::Inc);

        assert_eq!(gb.mem[0x1234], 0x99);
        assert_eq!(rr_to_u16(&mut gb, RHL), 0x1235);
    }

    #[test]
    fn ld_hl_ra_dec() {
        let mut gb = Gameboy::new();
        gb.mem[0x1234] = 0x05;
        gb.regs[RH] = 0x12;
        gb.regs[RL] = 0x34;
        gb.regs[RA] = 0x99;

        exec::ld_inc_dec(&mut gb, &Dst8::Id(RHL), &Src8::R8(RA), IncDec::Dec);

        assert_eq!(gb.mem[0x1234], 0x99);
        assert_eq!(rr_to_u16(&mut gb, RHL), 0x1233);
    }

    #[test]
    fn ld_ra_nn() {
        let mut gb = Gameboy::new();
        gb.mem[0x1234] = 0x99;
        gb.regs[RA] = 0x05;

        exec::ld(&mut gb, &Dst8::R8(RA), &Src8::IdNN(0x1234));

        assert_eq!(gb.regs[RA], 0x99);
    }

    #[test]
    fn ld_nn_ra() {
        let mut gb = Gameboy::new();
        gb.mem[0x1234] = 0x05;
        gb.regs[RA] = 0x99;

        exec::ld(&mut gb, &Dst8::IdNN(0x1234), &Src8::R8(RA));

        assert_eq!(gb.regs[RA], 0x99);
    }

    #[test]
    fn ldh_ra_rc() {
        let mut gb = Gameboy::new();
        gb.io_regs[0x03].store(0x99, Ordering::Relaxed);
        gb.regs[RC] = 0x03;
        gb.regs[RA] = 0x05;

        exec::ld(&mut gb, &Dst8::R8(RA), &Src8::IdFFRC);

        assert_eq!(gb.regs[RA], 0x99);
    }

    #[test]
    fn ldh_rc_ra() {
        let mut gb = Gameboy::new();
        gb.io_regs[0x03].store(0x05, Ordering::Relaxed);
        gb.regs[RC] = 0x03;
        gb.regs[RA] = 0x99;

        exec::ld(&mut gb, &Dst8::IdFFRC, &Src8::R8(RA));

        assert_eq!(gb.io_regs[0x03].load(Ordering::Relaxed), 0x99);
    }

    #[test]
    fn ldh_ra_n()  {
        let mut gb = Gameboy::new();
        gb.io_regs[0x03].store(0x99, Ordering::Relaxed);
        gb.regs[RA] = 0x05;

        exec::ld(&mut gb, &Dst8::R8(RA), &Src8::IdFF(0x03));

        assert_eq!(gb.regs[RA], 0x99);
    }

    #[test]
    fn ldh_n_ra()  {
        let mut gb = Gameboy::new();
        gb.io_regs[0x03].store(0x05, Ordering::Relaxed);
        gb.regs[RA] = 0x99;

        exec::ld(&mut gb, &Dst8::IdFF(3), &Src8::R8(RA));

        assert_eq!(gb.io_regs[0x03].load(Ordering::Relaxed), 0x99);
    }

    #[test]
    fn ld_r16_d16() {
        let mut gb = Gameboy::new();
        gb.regs[RD] = 0x05;
        gb.regs[RE] = 0x06;

        exec::ld_16(&mut gb, &Dst16::R16(RDE), &Src16::D16(0xbeef));

        assert_eq!(rr_to_u16(&mut gb, RDE), 0xbeef);
    }

    #[test]
    fn ld_rsp_d16() {
        let mut gb = Gameboy::new();
        gb.sp = 0x0506;

        exec::ld_16(&mut gb, &Dst16::RSP, &Src16::D16(0xbeef));

        assert_eq!(gb.sp, 0xbeef);
    }

    #[test]
    fn ld_nn_sp() {
        let mut gb = Gameboy::new();
        gb.mem[0x1234] = 0x05;
        gb.mem[0x1235] = 0x06;
        gb.sp = 0xbeef;

        exec::ld_16(&mut gb, &Dst16::IdNN(0x1234), &Src16::RSP);

        assert_eq!(gb.mem[0x1234], 0xbe);
        assert_eq!(gb.mem[0x1235], 0xef);
    }

    #[test]
    fn ld_sp_hl() {
        let mut gb = Gameboy::new();
        gb.regs[RH] = 0xbe;
        gb.regs[RL] = 0xef;
        gb.sp = 0x0506;

        exec::ld_16(&mut gb, &Dst16::RSP, &Src16::R16(RHL));

        assert_eq!(gb.sp, 0xbeef);
    }

    #[test]
    fn push() {
        let mut gb = Gameboy::new();
        gb.regs[RB] = 0xbe;
        gb.regs[RC] = 0xef;
        gb.sp = 0x0600;

        exec::push(&mut gb, &RBC);

        assert_eq!(gb.sp, 0x05fe);
        assert_eq!(gb.mem[0x05ff], 0xbe);
        assert_eq!(gb.mem[0x05fe], 0xef);
    }

    #[test]
    fn pop() {
        let mut gb = Gameboy::new();
        gb.regs[RB] = 0x05;
        gb.regs[RC] = 0x06;
        gb.sp = 0x05fe;
        gb.mem[0x05ff] = 0xbe;
        gb.mem[0x05fe] = 0xef;

        exec::pop(&mut gb, &RBC);

        assert_eq!(gb.sp, 0x0600);
        assert_eq!(gb.regs[RB], 0xbe);
        assert_eq!(gb.regs[RC], 0xef);
    }

    #[test]
    fn ld_hl_sp_r8_positive() {
        let mut gb = Gameboy::new();
        gb.regs[RH] = 0x05;
        gb.regs[RL] = 0x06;
        gb.sp = 0x0600;

        exec::ld_16(&mut gb, &Dst16::R16(RHL), &Src16::SPD8(7));

        assert_eq!(gb.regs[RH], 0x06);
        assert_eq!(gb.regs[RL], 0x07);
        assert_eq!(gb.regs[RF], 0);
    }

    #[test]
    fn ld_hl_sp_r8_negative() {
        let mut gb = Gameboy::new();
        gb.regs[RH] = 0x05;
        gb.regs[RL] = 0x06;
        gb.sp = 0x0600;

        exec::ld_16(&mut gb, &Dst16::R16(RHL), &Src16::SPD8(-7));

        assert_eq!(gb.regs[RH], 0x05);
        assert_eq!(gb.regs[RL], 0xf9);
        assert_eq!(gb.regs[RF], 0);
    }

    #[test]
    fn ld_hl_sp_r8_flag_c() {
        let mut gb = Gameboy::new();
        gb.regs[RH] = 0x05;
        gb.regs[RL] = 0x06;
        gb.sp = 0x0680;

        exec::ld_16(&mut gb, &Dst16::R16(RHL), &Src16::SPD8(-7));

        assert_eq!(gb.regs[RH], 0x06);
        assert_eq!(gb.regs[RL], 0x79);
        assert_eq!(gb.regs[RF], FLAG_C);
    }

    #[test]
    fn ld_hl_sp_r8_flag_h() {
        let mut gb = Gameboy::new();
        gb.regs[RH] = 0x05;
        gb.regs[RL] = 0x06;
        gb.sp = 0x0608;

        exec::ld_16(&mut gb,  &Dst16::R16(RHL), &Src16::SPD8(0x79));

        assert_eq!(gb.regs[RH], 0x06);
        assert_eq!(gb.regs[RL], 0x81);
        assert_eq!(gb.regs[RF], FLAG_H);
    }

    #[test]
    fn ld_hl_sp_r8_flag_hc() {
        let mut gb = Gameboy::new();
        gb.regs[RH] = 0x05;
        gb.regs[RL] = 0x06;
        gb.sp = 0x0688;

        exec::ld_16(&mut gb, &Dst16::R16(RHL), &Src16::SPD8(-7));

        assert_eq!(gb.regs[RH], 0x06);
        assert_eq!(gb.regs[RL], 0x81);
        assert_eq!(gb.regs[RF], FLAG_H | FLAG_C);
    }

    #[test]
    fn inc_r8() {
        let mut gb = Gameboy::new();
        gb.regs[RC] = 0x05;

        exec::inc_dec(&mut gb, &Dst8::R8(RC), IncDec::Inc);

        assert_eq!(gb.regs[RC], 0x06);
        assert_eq!(gb.regs[RF], 0);
    }

    #[test]
    fn inc_r8_overflow() {
        let mut gb = Gameboy::new();
        gb.regs[RC] = 0xff;

        exec::inc_dec(&mut gb, &Dst8::R8(RC), IncDec::Inc);

        assert_eq!(gb.regs[RC], 0x00);
        assert_eq!(gb.regs[RF], FLAG_Z | FLAG_H);
    }

    #[test]
    fn dec_r8() {
        let mut gb = Gameboy::new();
        gb.regs[RC] = 0x05;

        exec::inc_dec(&mut gb, &Dst8::R8(RC), IncDec::Dec);

        assert_eq!(gb.regs[RC], 0x04);
        assert_eq!(gb.regs[RF], FLAG_N | FLAG_H);
    }

    #[test]
    fn dec_r8_z() {
        let mut gb = Gameboy::new();
        gb.regs[RC] = 0x01;

        exec::inc_dec(&mut gb, &Dst8::R8(RC), IncDec::Dec);

        assert_eq!(gb.regs[RC], 0x00);
        assert_eq!(gb.regs[RF], FLAG_Z | FLAG_N | FLAG_H);
    }

    #[test]
    fn dec_r8_underflow() {
        let mut gb = Gameboy::new();
        gb.regs[RC] = 0x00;

        exec::inc_dec(&mut gb, &Dst8::R8(RC), IncDec::Dec);

        assert_eq!(gb.regs[RC], 0xff);
        assert_eq!(gb.regs[RF], FLAG_N);
    }

    #[test]
    fn inc_id_hl() {
        let mut gb = Gameboy::new();
        gb.mem[0x1234] = 0x05;
        gb.regs[RH] = 0x12;
        gb.regs[RL] = 0x34;

        exec::inc_dec(&mut gb, &Dst8::Id(RHL), IncDec::Inc);

        assert_eq!(gb.mem[0x1234], 0x06);
        assert_eq!(gb.regs[RF], 0);
    }

    #[test]
    fn inc_id_hl_overflow() {
        let mut gb = Gameboy::new();
        gb.mem[0x1234] = 0xff;
        gb.regs[RH] = 0x12;
        gb.regs[RL] = 0x34;

        exec::inc_dec(&mut gb, &Dst8::Id(RHL), IncDec::Inc);

        assert_eq!(gb.mem[0x1234], 0x00);
        assert_eq!(gb.regs[RF], FLAG_Z | FLAG_H);
    }

    #[test]
    fn dec_id_hl() {
        let mut gb = Gameboy::new();
        gb.mem[0x1234] = 0x05;
        gb.regs[RH] = 0x12;
        gb.regs[RL] = 0x34;

        exec::inc_dec(&mut gb, &Dst8::Id(RHL), IncDec::Dec);

        assert_eq!(gb.mem[0x1234], 0x04);
        assert_eq!(gb.regs[RF], FLAG_N | FLAG_H);
    }

    #[test]
    fn dec_id_hl_z() {
        let mut gb = Gameboy::new();
        gb.mem[0x1234] = 0x01;
        gb.regs[RH] = 0x12;
        gb.regs[RL] = 0x34;

        exec::inc_dec(&mut gb, &Dst8::Id(RHL), IncDec::Dec);

        assert_eq!(gb.mem[0x1234], 0x00);
        assert_eq!(gb.regs[RF], FLAG_Z | FLAG_N | FLAG_H);
    }

    #[test]
    fn dec_id_hl_underflow() {
        let mut gb = Gameboy::new();
        gb.mem[0x1234] = 0x00;
        gb.regs[RH] = 0x12;
        gb.regs[RL] = 0x34;

        exec::inc_dec(&mut gb, &Dst8::Id(RHL), IncDec::Dec);

        assert_eq!(gb.mem[0x1234], 0xFF);
        assert_eq!(gb.regs[RF], FLAG_N);
    }

    #[test]
    fn add_r8() {
        let mut gb = Gameboy::new();
        gb.regs[RF] = FLAG_C;
        gb.regs[RA] = 0x05;
        gb.regs[RE] = 0x94;

        exec::add_sub(&mut gb, &Src8::R8(RE), AddSub::Add, CarryMode::NoCarry);

        assert_eq!(gb.regs[RA], 0x99);
        assert_eq!(gb.regs[RF], 0);
    }

    #[test]
    fn adc_r8() {
        let mut gb = Gameboy::new();
        gb.regs[RF] = FLAG_C;
        gb.regs[RA] = 0x05;
        gb.regs[RE] = 0x94;

        exec::add_sub(&mut gb, &Src8::R8(RE), AddSub::Add, CarryMode::WithCarry);

        assert_eq!(gb.regs[RA], 0x9A);
        assert_eq!(gb.regs[RF], 0);
    }

    #[test]
    fn sub_r8() {
        let mut gb = Gameboy::new();
        gb.regs[RF] = FLAG_C;
        gb.regs[RA] = 0x99;
        gb.regs[RE] = 0x05;

        exec::add_sub(&mut gb, &Src8::R8(RE), AddSub::Sub, CarryMode::NoCarry);

        assert_eq!(gb.regs[RA], 0x94);
        assert_eq!(gb.regs[RF], FLAG_N);
    }

    #[test]
    fn sbc_r8() {
        let mut gb = Gameboy::new();
        gb.regs[RF] = FLAG_C;
        gb.regs[RA] = 0x99;
        gb.regs[RE] = 0x05;

        exec::add_sub(&mut gb, &Src8::R8(RE), AddSub::Sub, CarryMode::WithCarry);

        assert_eq!(gb.regs[RA], 0x93);
        assert_eq!(gb.regs[RF], FLAG_N);
    }

    #[test]
    fn cp_eq() {
        let mut gb = Gameboy::new();
        gb.regs[RA] = 0x99;
        gb.regs[RE] = 0x99;

        exec::cp(&mut gb, &Src8::R8(RE));

        assert_eq!(gb.regs[RF], FLAG_Z | FLAG_N);
    }

    #[test]
    fn cp_lt() {
        let mut gb = Gameboy::new();
        gb.regs[RA] = 0x99;
        gb.regs[RE] = 0x9a;

        exec::cp(&mut gb, &Src8::R8(RE));

        assert_eq!(gb.regs[RF], FLAG_N | FLAG_H | FLAG_C);
    }

    #[test]
    fn cp_gt() {
        let mut gb = Gameboy::new();
        gb.regs[RA] = 0x9a;
        gb.regs[RE] = 0x99;

        exec::cp(&mut gb, &Src8::R8(RE));

        assert_eq!(gb.regs[RF], FLAG_N);
    }
}