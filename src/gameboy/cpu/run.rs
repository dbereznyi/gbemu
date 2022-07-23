use std::thread;
use std::fmt;
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

            if io_if & INT_VBLANK > 0 {
                gb.pc = 0x0040;
                gb.io_ports.and(IO_IF, !INT_VBLANK);
            } else if io_if & INT_LCDC > 0 {
                gb.pc = 0x0048;
                gb.io_ports.and(IO_IF, !INT_LCDC);
            } else if io_if & INT_TIMER > 0 {
                gb.pc = 0x0050;
                gb.io_ports.and(IO_IF, !INT_TIMER);
            } else if io_if & INT_SERIAL > 0 {
                gb.pc = 0x0058;
                gb.io_ports.and(IO_IF, !INT_SERIAL);
            } else if io_if & INT_HILO > 0 {
                gb.pc = 0x0060;
                gb.io_ports.and(IO_IF, !INT_HILO);
            }

            gb.ime.store(false, Ordering::Relaxed);
            let (mutex, _) = &*gb.interrupt_received;
            let mut interrupted = mutex.lock().unwrap();
            *interrupted = false;
        }

        if gb.breakpoints.contains(&gb.pc) {
            gb.step_mode = true;
        }

        if gb.step_mode {
            loop {
                println!("==> ${:0>4X}: {}",
                         gb.pc, 
                         decode(gb, gb.pc).map(|i| i.to_string()).unwrap_or("".to_string()));
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
        }

        step(gb).unwrap();

        let elapsed = cpu_start.elapsed();
        let expected = Duration::from_micros(gb.cycles); 
        if expected > elapsed {
            thread::sleep(expected - elapsed);
        }
        // TODO This doesn't really work well when the CPU is halted. Need to do something a bit
        // different.
        debug_info.actual_time_micros.store(elapsed.as_micros() as u64, Ordering::Relaxed);
        debug_info.expected_time_micros.store(expected.as_micros() as u64, Ordering::Relaxed);
    }
}
