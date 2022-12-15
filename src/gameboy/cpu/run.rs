use std::thread;
use std::fmt;
use std::time::{Duration, Instant};
use std::sync::{Arc};
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::{JoinHandle, Thread};
use std::io::{self, Write};
use crate::gameboy::gameboy::{*};
use crate::gameboy::debug::{*};
use crate::gameboy::debug_info::{DebugInfoCpu};
use crate::gameboy::cpu::step::{step, decode};
use crate::gameboy::cpu::exec::{push_pc};
use crate::gameboy::utils::{sleep_precise};

pub fn run_cpu<T>(gb: &mut Gameboy, debug_info: DebugInfoCpu, components: &[JoinHandle<T>]) {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

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

        let cpu_start = Instant::now();

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

        if gb.debug.breakpoints.contains(&gb.pc) || gb.pc == gb.debug.over_ret_addr {
            gb.debug.step_mode.store(true, Ordering::Release);
        }

        if gb.debug.step_mode.load(Ordering::Acquire) {
            loop {
                println!("==> ${:0>4X}: {}",
                         gb.pc, 
                         decode(gb, gb.pc).map(|i| i.to_string()).unwrap_or("".to_string()));
                print!("> ");
                stdout.flush().unwrap();
                let mut line = String::new();
                stdin.read_line(&mut line).unwrap();
                match DebugCmd::new(&line) {
                    Ok(cmd) => {
                        match cmd.run(gb) {
                            Ok(exit_prompt_loop) => {
                                if exit_prompt_loop { break; }
                            },
                            Err(err) => eprintln!("{}", err),
                        }
                    },
                    Err(err) => eprintln!("{}", err),
                }
            }

            unpark_components(components);
        }

        let cycles_start = gb.cycles;
        step(gb).unwrap();

        let elapsed = cpu_start.elapsed();
        let expected = Duration::from_micros(gb.cycles - cycles_start); 
        if expected > elapsed {
            sleep_precise(expected - elapsed);
        }
        debug_info.actual_time_nanos.store(cpu_start.elapsed().as_nanos() as u64, Ordering::Relaxed);
        debug_info.expected_time_nanos.store(expected.as_nanos() as u64, Ordering::Relaxed);
    }
}

fn unpark_components<T>(components: &[JoinHandle<T>]) {
    for component in components.iter() {
        component.thread().unpark();
    }
}
