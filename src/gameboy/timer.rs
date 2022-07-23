use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::thread;
use std::time::{Duration};
use crate::gameboy::gameboy::{*};

const CLOCK_4096_HZ_PULSE: Duration    = Duration::from_nanos(1_000_000_000 / 4096);
const CLOCK_262_144_HZ_PULSE: Duration = Duration::from_nanos(1_000_000_000 / 262_144);
const CLOCK_65_536_HZ_PULSE: Duration  = Duration::from_nanos(1_000_000_000 / 65_536);
const CLOCK_16_384_HZ_PULSE: Duration  = Duration::from_nanos(1_000_000_000 / 16_384);

pub struct Timer {
    pub io_ports: Arc<IoPorts>,
    pub interrupt_received: Arc<(Mutex<bool>, Condvar)>,
    pub ime: Arc<AtomicBool>,
    pub timer_enabled: Arc<(Mutex<bool>, Condvar)>,
}

pub fn run_timer(timer: &mut Timer) {
    let io_ports = &timer.io_ports;

    loop {
        if io_ports.read(IO_TAC) & TAC_ENABLE == 0 {
            let (mutex, cvar) = &*timer.timer_enabled;
            let mut enabled = mutex.lock().unwrap();
            while !*enabled {
                enabled = cvar.wait(enabled).unwrap();
            }
        }

        let delay = match io_ports.read(IO_TAC) & TAC_CLOCK_SELECT {
            0 => CLOCK_4096_HZ_PULSE,
            1 => CLOCK_262_144_HZ_PULSE,
            2 => CLOCK_65_536_HZ_PULSE,
            3 => CLOCK_16_384_HZ_PULSE,
            _ => panic!("Invalid clock select"),
        };
        
        thread::sleep(delay);

        io_ports.add(IO_TIMA, 1);

        let timer_overflow = io_ports.read(IO_TIMA) == 0;
        if timer_overflow {
            io_ports.write(IO_TIMA, io_ports.read(IO_TMA));
        }

        if timer.ime.load(Ordering::Relaxed) && io_ports.read(IO_IE) & INT_TIMER > 0 && timer_overflow {
            io_ports.or(IO_IF, INT_TIMER);
            let (mutex, cvar) = &*timer.interrupt_received;
            let mut interrupted = mutex.lock().unwrap();
            *interrupted = true;
            cvar.notify_one();
        }
    }
}
