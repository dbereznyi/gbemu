use std::thread;
use std::time::{Duration, Instant};
use std::sync::atomic::{Ordering};
use crate::gameboy::gameboy::{*};
use crate::gameboy::cpu::step::{step};
use crate::gameboy::cpu::exec::{push_pc};

pub fn run_cpu(gb: &mut Gameboy) {
    loop {
        // If CPU is halted, just wait until an interrupt wakes us up
        // CPU will only be interrupted if IME is set and corresponding IE bit is set,
        // so we will always get an interrupt we can process from this
        if gb.halted.load(Ordering::Relaxed) {
            let (mutex, cvar) = &*gb.interrupt_received;
            let mut interrupted = mutex.lock().unwrap();
            while !*interrupted {
                interrupted = cvar.wait(interrupted).unwrap();
            }
            *interrupted = false;
        } 
        
        let cycles_start = gb.cycles;
        let start = Instant::now();

        // Handle any interrupts that may have happened
        let mut io_ports = *gb.io_ports.lock().unwrap();
        if gb.ime.load(Ordering::Relaxed) && io_ports[IO_IF] > 0 {
            push_pc(gb);

            if io_ports[IO_IF] & VBLANK > 0 {
                gb.pc = 0x0040;
                io_ports[IO_IF] &= !VBLANK;
            } else if io_ports[IO_IF] & LCDC > 0 {
                gb.pc = 0x0048;
                io_ports[IO_IF] &= !LCDC;
            } else if io_ports[IO_IF] & TIMER > 0 {
                gb.pc = 0x0050;
                io_ports[IO_IF] &= !TIMER;
            } else if io_ports[IO_IF] & SERIAL > 0 {
                gb.pc = 0x0058;
                io_ports[IO_IF] &= !SERIAL;
            } else if io_ports[IO_IF] & HI_TO_LOW > 0 {
                gb.pc = 0x0060;
                io_ports[IO_IF] &= !HI_TO_LOW;
            }

            gb.ime.store(false, Ordering::Relaxed);
            let (mutex, _) = &*gb.interrupt_received;
            let mut interrupted = mutex.lock().unwrap();
            *interrupted = false;
        }
        drop(io_ports);

        step(gb);

        // Figure out how much time we took to execute this instruction, and pad out the time if we
        // took less than was needed
        let cycles_elapsed = (gb.cycles - cycles_start) as u32;
        let actual_runtime = start.elapsed();
        let expected_runtime = Duration::new(0, 1000 * cycles_elapsed);
        if expected_runtime > actual_runtime {
            thread::sleep(expected_runtime - actual_runtime);
        }
        //println!("Expected: {:#?}, Actual: {:#?}", expected_runtime, actual_runtime);
    }
}

