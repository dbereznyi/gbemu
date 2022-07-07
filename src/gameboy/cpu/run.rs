use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc};
use std::sync::atomic::{AtomicU64, Ordering};
use crate::gameboy::gameboy::{*};
use crate::gameboy::cpu::step::{step};
use crate::gameboy::cpu::exec::{push_pc};

pub fn run_cpu(
    gb: &mut Gameboy,
    cpu_actual_time_micros: Arc<AtomicU64>,
    cpu_expected_time_micros: Arc<AtomicU64>) {
    let cpu_start = Instant::now();

    loop {
        // If CPU is halted, just wait until an interrupt wakes us up.
        // CPU will only be interrupted if IME is set and corresponding IE bit is set,
        // so we will always get an interrupt we can process from this.
        if gb.halted.load(Ordering::Relaxed) {
            let (mutex, cvar) = &*gb.interrupt_received;
            let mut interrupted = mutex.lock().unwrap();
            while !*interrupted {
                interrupted = cvar.wait(interrupted).unwrap();
            }
            *interrupted = false;
            gb.halted.store(false, Ordering::Relaxed);
        } 

        let io_if = gb.io_ports.read(IO_IF);
        if gb.ime.load(Ordering::Relaxed) && io_if > 0 {
            push_pc(gb);

            if io_if & VBLANK > 0 {
                gb.pc = 0x0040;
                gb.io_ports.and(IO_IF, !VBLANK);
            } else if io_if & LCDC > 0 {
                gb.pc = 0x0048;
                gb.io_ports.and(IO_IF, !LCDC);
            } else if io_if & TIMER > 0 {
                gb.pc = 0x0050;
                gb.io_ports.and(IO_IF, !TIMER);
            } else if io_if & SERIAL > 0 {
                gb.pc = 0x0058;
                gb.io_ports.and(IO_IF, !SERIAL);
            } else if io_if & HI_TO_LOW > 0 {
                gb.pc = 0x0060;
                gb.io_ports.and(IO_IF, !HI_TO_LOW);
            }

            gb.ime.store(false, Ordering::Relaxed);
            let (mutex, _) = &*gb.interrupt_received;
            let mut interrupted = mutex.lock().unwrap();
            *interrupted = false;
        }

        step(gb);

        let elapsed = cpu_start.elapsed();
        let expected = Duration::from_micros(gb.cycles); 
        if expected > elapsed {
            thread::sleep(expected - elapsed);
        }
        cpu_actual_time_micros.store(elapsed.as_micros() as u64, Ordering::Relaxed);
        cpu_expected_time_micros.store(expected.as_micros() as u64, Ordering::Relaxed);
    }
}
