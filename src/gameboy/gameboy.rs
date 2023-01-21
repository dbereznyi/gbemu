use std::fmt;
use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use crate::gameboy::cartridge::{*};

/// 8-bit register.
pub type R = usize;
/// 16-bit register, i.e. a pair of 8-bit registers used together.
pub type RR = (usize, usize);

pub const RB: R = 0;
pub const RC: R = 1;
pub const RD: R = 2;
pub const RE: R = 3;
pub const RH: R = 4;
pub const RL: R = 5;
pub const RF: R = 6;
pub const RA: R = 7;

pub const RAF: RR = (RA, RF);
pub const RBC: RR = (RB, RC);
pub const RDE: RR = (RD, RE);
pub const RHL: RR = (RH, RL);

pub const FLAG_Z: u8 = 0b10000000;
pub const FLAG_N: u8 = 0b01000000;
pub const FLAG_H: u8 = 0b00100000;
pub const FLAG_C: u8 = 0b00010000;

// IO port/register aliases, relative to 0xff00.
pub const IO_P1: usize   = 0x00;
pub const IO_TIMA: usize = 0x05;
pub const IO_TMA: usize  = 0x06;
pub const IO_TAC: usize  = 0x07;
pub const IO_IF: usize   = 0x0f;
pub const IO_LCDC: usize = 0x40; 
pub const IO_STAT: usize = 0x41;
pub const IO_SCY: usize  = 0x42; 
pub const IO_SCX: usize  = 0x43; 
pub const IO_LY: usize   = 0x44;
pub const IO_LYC: usize  = 0x45;
pub const IO_BGP: usize  = 0x47;
pub const IO_OBP0: usize = 0x48;
pub const IO_OBP1: usize = 0x49;
pub const IO_WY: usize   = 0x4a;
pub const IO_WX: usize   = 0x4b;
// In the memory map this is at 0xffff, but to simplify things internally we store this at the end
// of the io_ports array
pub const IO_IE: usize   = 0x4c;

pub const P1_OUT: u8     = 0b0011_0000;
pub const P1_P14_OUT: u8 = 0b0001_0000;
pub const P1_P15_OUT: u8 = 0b0010_0000;

pub const TAC_CLOCK_SELECT: u8 = 0b0000_0011;
pub const TAC_ENABLE: u8       = 0b0000_0100;

pub const CONTROLLER_DATA_P14: u8   = 0b1111_0000;
pub const CONTROLLER_DATA_P15: u8   = 0b0000_1111;
pub const CONTROLLER_DATA_A: u8     = 0b0000_0001;
pub const CONTROLLER_DATA_B: u8     = 0b0000_0010;
pub const CONTROLLER_DATA_SE: u8    = 0b0000_0100;
pub const CONTROLLER_DATA_ST: u8    = 0b0000_1000;
pub const CONTROLLER_DATA_RIGHT: u8 = 0b0001_0000;
pub const CONTROLLER_DATA_LEFT: u8  = 0b0010_0000;
pub const CONTROLLER_DATA_UP: u8    = 0b0100_0000;
pub const CONTROLLER_DATA_DOWN: u8  = 0b1000_0000;

pub const LCDC_ON: u8           = 0b1000_0000;
pub const LCDC_WIN_TILE_MAP: u8 = 0b0100_0000;
pub const LCDC_WIN_DISP: u8     = 0b0010_0000;
pub const LCDC_TILE_DATA: u8    = 0b0001_0000;
pub const LCDC_BG_TILE_MAP: u8  = 0b0000_1000;
pub const LCDC_OBJ_SIZE: u8     = 0b0000_0100;
pub const LCDC_OBJ_DISP: u8     = 0b0000_0010;
pub const LCDC_BG_DISP: u8      = 0b0000_0001;

pub const STAT_INT_LYC: u8 = 0b0100_0000;
pub const STAT_INT_M10: u8 = 0b0010_0000;
pub const STAT_INT_M01: u8 = 0b0001_0000;
pub const STAT_INT_M00: u8 = 0b0000_1000;
pub const STAT_LYC_SET: u8 = 0b0000_0100;
pub const STAT_MODE: u8    = 0b0000_0011;

pub const STAT_MODE_HBLANK: u8   = 0b0000_0000;
pub const STAT_MODE_VBLANK: u8   = 0b0000_0001;
pub const STAT_MODE_OAM: u8      = 0b0000_0010;
pub const STAT_MODE_TRANSFER: u8 = 0b0000_0011;

// Interrupt flags, used for IF and IE registers.
pub const INT_VBLANK: u8      = 0b0000_0001;
pub const INT_LCDC: u8        = 0b0000_0010;
pub const INT_TIMER: u8       = 0b0000_0100;
pub const INT_SERIAL: u8      = 0b0000_1000;
pub const INT_HILO: u8        = 0b0001_0000;

pub struct IoPorts {
    io_ports: [AtomicU8; 0x4d],
}

impl IoPorts {
    pub fn new(io_ports: [AtomicU8; 0x4d]) -> Self {
        Self {
            io_ports
        }
    }

    pub fn read(&self, port: usize) -> u8 {
        self.io_ports[port].load(Ordering::Relaxed)
    }

    pub fn write(&self, port: usize, value: u8) {
        self.io_ports[port].store(value, Ordering::Relaxed)
    }

    pub fn and(&self, port: usize, value: u8) {
        self.io_ports[port].fetch_and(value, Ordering::Relaxed);
    }

    pub fn or(&self, port: usize, value: u8) {
        self.io_ports[port].fetch_or(value, Ordering::Relaxed);
    }

    pub fn xor(&self, port: usize, value: u8) {
        self.io_ports[port].fetch_xor(value, Ordering::Relaxed);
    }

    pub fn add(&self, port: usize, value: u8) {
        self.io_ports[port].fetch_add(value, Ordering::Relaxed);
    }
}

pub struct Debug {
    pub step_mode: Arc<AtomicBool>,
    pub breakpoints: Vec<u16>,
    pub over_ret_addr: u16,
    pub stack_base: u16,
}

impl Debug {
    pub fn new() -> Self {
        Self {
            step_mode: Arc::new(AtomicBool::new(false)),
            breakpoints: vec!(),
            over_ret_addr: 0x0000,
            stack_base: 0xfffe,
        }
    }
}

pub struct Gameboy {
    wram: Box<[u8; 0x2000]>,
    pub vram: Arc<Mutex<[u8; 0x2000]>>,
    pub oam: Arc<Mutex<[u8; 0xa0]>>,
    pub io_ports: Arc<IoPorts>,
    hram: Box<[u8; 0x7f]>,
    cartridge: Cartridge,

    pub cycles: u64, 
    pub pc: u16,
    pub sp: u16,
    pub regs: [u8; 8], 
    pub ime: Arc<AtomicBool>,
    pub halted: Arc<AtomicBool>,
    pub stopped: Arc<AtomicBool>,

    pub debug: Debug,

    /// CPU can wait on this variable to sleep until interrupted.
    pub interrupt_received: Arc<(Mutex<bool>, Condvar)>,
    /// Timer can wait on this variable to sleep until timer is enabled.
    pub timer_enabled: Arc<(Mutex<bool>, Condvar)>,
    /// Which buttons are currently being pressed.
    /// Like the actual Gameboy P1 register, 1 means not pressed and 0 means pressed.
    pub controller_data: Arc<AtomicU8>,
    /// Pixel data to be drawn to the screen.
    pub screen: Arc<Mutex<[[(u8, u8, u8); 160]; 144]>>,
}

impl Gameboy {
    pub fn new(cartridge: Cartridge) -> Self {
        const ZERO_AU8: AtomicU8 = AtomicU8::new(0);
        let io_ports = IoPorts::new([ZERO_AU8; 0x4d]);
        io_ports.write(IO_P1, 0xcf); // not sure if this is accurate but BGB seems to do it
        io_ports.write(IO_LCDC, 0x91);
        io_ports.write(IO_BGP, 0xfc);
        io_ports.write(IO_OBP0, 0xff);
        io_ports.write(IO_OBP1, 0xff);

        Self {
            wram: Box::new([0; 0x2000]),
            vram: Arc::new(Mutex::new([0; 0x2000])),
            oam: Arc::new(Mutex::new([0; 0xa0])),
            io_ports: Arc::new(io_ports),
            hram: Box::new([0; 0x7f]),
            cartridge: cartridge,

            debug: Debug::new(),

            cycles: 0,
            pc: 0x0100, 
            sp: 0xfffe,
            regs: [0; 8],
            ime: Arc::new(AtomicBool::new(false)),
            halted: Arc::new(AtomicBool::new(false)),
            stopped: Arc::new(AtomicBool::new(false)),

            interrupt_received: Arc::new((Mutex::new(false), Condvar::new())),
            timer_enabled: Arc::new((Mutex::new(false), Condvar::new())),
            controller_data: Arc::new(AtomicU8::new(0xff)),
            screen: Arc::new(Mutex::new([[(0,0,0); 160]; 144])),
        }
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7fff => {
                self.cartridge.read_rom(addr)
            },
            0x8000..=0x9fff => {
                let vram = self.vram.lock().unwrap();
                vram[(addr - 0x8000) as usize]
            },
            0xa000..=0xbfff => {
                self.cartridge.read_ram(addr)
            },
            0xc000..=0xdfff => {
                self.wram[(addr - 0xc000) as usize] 
            },
            0xe000..=0xfdff => {
                self.wram[(addr - 0xe000) as usize]
            },
            0xfe00..=0xfe9f => {
                let oam = self.oam.lock().unwrap();
                oam[(addr - 0xfe00) as usize]
            },
            0xfea0..=0xfeff => {
                println!("Warning: attempt to read from invalid memory ${addr:0>4x}");
                0xff
            },
            0xff00..=0xff4b => {
                let port = (addr - 0xff00) as usize;
                self.io_ports.read(port)
            },
            0xff4c..=0xff7f => {
                println!("Warning: attempt to read from invalid memory ${addr:0>4x}");
                0xff
            },
            0xff80..=0xfffe => {
                self.hram[(addr - 0xff80) as usize]
            },
            0xffff => {
                self.io_ports.read(IO_IE)
            },
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7fff => {
                self.cartridge.write_rom(addr, value)
            },
            0x8000..=0x9fff => {
                let mut vram = self.vram.lock().unwrap();
                vram[(addr - 0x8000) as usize] = value
            },
            0xa000..=0xbfff => {
                self.cartridge.write_ram(addr, value)
            },
            0xc000..=0xdfff => {
                self.wram[(addr - 0xc000) as usize] = value
            },
            0xe000..=0xfdff => {
                self.wram[(addr - 0xe000) as usize] = value
            },
            0xfe00..=0xfe9f => {
                let mut oam = self.oam.lock().unwrap();
                oam[(addr - 0xfe00) as usize] = value
            },
            0xfea0..=0xfeff => {
                println!("Warning: attempt to write to invalid memory ${addr:0>4x}")
            },
            0xff00..=0xff4b => {
                let port = (addr - 0xff00) as usize;
                match port {
                    IO_P1 => {
                        let output_select = value & P1_OUT;
                        let cont_data = self.controller_data.load(Ordering::Relaxed);
                        let output =
                            if output_select & P1_P15_OUT == 0 {
                                cont_data & CONTROLLER_DATA_P15
                            } else if output_select & P1_P14_OUT == 0 {
                                (cont_data & CONTROLLER_DATA_P14) >> 4
                            } else {
                                // TODO does P1 actually output 1s here if no output is selected?
                                0b0000_1111
                            };

                        self.io_ports.write(IO_P1, output_select | output);
                    },
                    IO_TAC => {
                        self.io_ports.write(IO_TAC, value);

                        if value & TAC_ENABLE > 0 {
                            let (mutex, cvar) = &*self.timer_enabled;
                            let mut enabled = mutex.lock().unwrap();
                            *enabled = true;
                            cvar.notify_one();
                        }
                    },
                    _ => self.io_ports.write(port, value),
                }
            },
            0xff4c..=0xff7f => {
                println!("Warning: attempt to write to invalid memory ${addr:0>4x}")
            },
            0xff80..=0xfffe => {
                self.hram[(addr - 0xff80) as usize] = value
            },
            0xffff => {
                self.io_ports.write(IO_IE, value)
            },
        }
    }
}

impl fmt::Display for Gameboy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, 
            concat!(
                "PC: {:0>4X}, SP: {:0>X}, cycles: {:0>6}\n",
                "Z: {:2}, N: {:2}, H: {:2}, C: {:2}\n",
                "A: {:0>2X}, B: {:0>2X}, D: {:0>2X}, H: {:0>2X}\n",
                "F: {:0>2X}, C: {:0>2X}, E: {:0>2X}, L: {:0>2X}\n",
                "LY: {:0>2X}, LCDC: {:0>2X}, STAT: {:0>2X}\n",
                "P1: {:0>2X}",
            ),
            self.pc, self.sp, self.cycles,
            (self.regs[RF] & FLAG_Z) >> 7, (self.regs[RF] & FLAG_N) >> 6,
            (self.regs[RF] & FLAG_H) >> 5, (self.regs[RF] & FLAG_C) >> 4,
            self.regs[RA], self.regs[RB], self.regs[RD], self.regs[RH],
            self.regs[RF], self.regs[RC], self.regs[RE], self.regs[RL],
            self.io_ports.read(IO_LY), self.io_ports.read(IO_LCDC), self.io_ports.read(IO_STAT),
            self.io_ports.read(IO_P1),
        )
    }
}

/// Converts a register pair alias into a u16.
pub fn rr_to_u16(gb: &Gameboy, reg_pair: RR) -> u16 {
    let upper = (gb.regs[reg_pair.0] as u16) << 8;
    let lower = gb.regs[reg_pair.1] as u16;
    upper ^ lower
}
