use std::thread;
use std::time::{Duration, Instant};
use std::sync::{Arc};
use std::sync::atomic::{AtomicU64, Ordering};
use std::io::{self, Write};
use crate::gameboy::gameboy::{*};
use crate::gameboy::debug::{*};
use crate::gameboy::debug_info::{DebugInfoCpu};
use crate::gameboy::cpu::step::{step, decode};
use crate::gameboy::cpu::exec::{push_pc};

pub fn run_cpu(gb: &mut Gameboy, debug_info: DebugInfoCpu) {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
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
            } else if io_if & P1_NEG_EDGE > 0 {
                gb.pc = 0x0060;
                gb.io_ports.and(IO_IF, !P1_NEG_EDGE);
            }

            gb.ime.store(false, Ordering::Relaxed);
            let (mutex, _) = &*gb.interrupt_received;
            let mut interrupted = mutex.lock().unwrap();
            *interrupted = false;
        }

        step(gb);

        if gb.breakpoints.contains(&gb.pc) {
            gb.step_mode = true;
        }

        if gb.step_mode {
            loop {
                println!("==> ${:0>4X}: {}", gb.pc, decode(gb).to_str());
                print!("> ");
                stdout.flush().unwrap();
                let mut line = String::new();
                stdin.read_line(&mut line).unwrap();
                let cmd = DebugCmd::new(&line);
                match cmd {
                    Result::Ok(cmd) => {
                        let should_break = cmd.run(gb);
                        if should_break { break; }
                    },
                    Result::Err(err) => eprintln!("{}", err),
                }
            }

            continue;
        }

        let elapsed = cpu_start.elapsed();
        let expected = Duration::from_micros(gb.cycles); 
        if expected > elapsed {
            thread::sleep(expected - elapsed);
        }
        debug_info.actual_time_micros.store(elapsed.as_micros() as u64, Ordering::Relaxed);
        debug_info.expected_time_micros.store(expected.as_micros() as u64, Ordering::Relaxed);
    }
}
