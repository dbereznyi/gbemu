use std::fmt;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU8, Ordering};

// Registers are referred to by indexing into Gameboy.regs
/// The type of an 8-bit register
pub type R = usize;
/// The type of a 16-bit register, i.e. a pair of 8-bit registers used together
pub type RR = (usize, usize);

// 8-bit register aliases
pub const RB: R = 0;
pub const RC: R = 1;
pub const RD: R = 2;
pub const RE: R = 3;
pub const RH: R = 4;
pub const RL: R = 5;
pub const RF: R = 6;
pub const RA: R = 7;

// 16-bit register aliases
pub const RAF: RR = (RA, RF);
pub const RBC: RR = (RB, RC);
pub const RDE: RR = (RD, RE);
pub const RHL: RR = (RH, RL);

// Flag aliases
pub const FLAG_Z: u8 = 0b10000000;
pub const FLAG_N: u8 = 0b01000000;
pub const FLAG_H: u8 = 0b00100000;
pub const FLAG_C: u8 = 0b00010000;

// IO port/register aliases, relative to 0xff00
pub const IO_IF: usize   = 0x0f;
pub const IO_LCDC: usize = 0x40; 
pub const IO_STAT: usize = 0x41;
pub const IO_SCY: usize  = 0x42; 
pub const IO_SCX: usize  = 0x43; 

// LCDC settings
pub const LCDC_ON: u8           = 0b1000_0000;
pub const LCDC_WIN_TILE_MAP: u8 = 0b0100_0000;
pub const LCDC_WIN_DISP: u8     = 0b0010_0000;
pub const LCDC_TILE_DATA: u8    = 0b0001_0000;
pub const LCDC_BG_TILE_MAP: u8  = 0b0000_1000;
pub const LCDC_OBJ_SIZE: u8     = 0b0000_0100;
pub const LCDC_OBJ_DISP: u8     = 0b0000_0010;
pub const LCDC_BG_WIN_DISP: u8  = 0b0000_0001;

// STAT settings
pub const STAT_INT_LYC: u8 = 0b0100_0000;
pub const STAT_INT_M10: u8 = 0b0010_0000;
pub const STAT_INT_M01: u8 = 0b0001_0000;
pub const STAT_INT_M00: u8 = 0b0000_1000;
pub const STAT_LYC_SET: u8 = 0b0000_0100;
pub const STAT_MODE: u8    = 0b0000_0011;

// STAT modes
pub const STAT_MODE_HBLANK: u8   = 0b0000_0000;
pub const STAT_MODE_VBLANK: u8   = 0b0000_0001;
pub const STAT_MODE_OAM: u8      = 0b0000_0010;
pub const STAT_MODE_TRANSFER: u8 = 0b0000_0011;

// Interrupt flags (used for IF and IE registers)
pub const VBLANK: u8 = 0b0000_0001;
pub const LCDC: u8   = 0b0000_0010;
pub const TIMER: u8  = 0b0000_0100;
pub const H2L: u8    = 0b0000_1000;

pub struct Gameboy {
    /// Working RAM, accessible by CPU only
    pub wram: Box<[u8; 0x2000]>,
    /// Video RAM, accessible by CPU and PPU (but not at same time)
    pub vram: Arc<Mutex<[u8; 0x2000]>>,
    /// Object (sprite) Attribute Memory, used by PPU and can be written to via DMA transfer
    pub oam: Arc<Mutex<[u8; 0xa0]>>,
    /// IO Ports and hardware control registers
    pub io_ports: Arc<Mutex<[u8; 0x4c]>>,
    /// Internal RAM, used e.g. for stack
    pub iram: Box<[u8; 0x7f]>,
    /// Interrupt Enable IO register
    pub io_ie: Arc<AtomicU8>,
    /// 32kB ROM. Mapping into memory depends on ROM type
    pub rom: Box<[u8; 0x8000]>,

    /// Elapsed machine cycles
    pub cycles: u64, 
    pub pc: u16,
    pub sp: u16,
    /// Registers A, B, C, D, E, F, H, L
    pub regs: [u8; 8], 
    /// Interrupt Master Enable
    pub ime: bool,
    pub halted: bool,
    pub stopped: bool,

    /// Holds pixel data to be drawn to the screen
    pub screen: Arc<Mutex<[[u8; 160]; 144]>>,
}

impl Gameboy {
    /// Creates a new Gameboy struct.
    pub fn new() -> Gameboy {
        Gameboy {
            wram: Box::new([0; 0x2000]),
            vram: Arc::new(Mutex::new([0; 0x2000])),
            oam: Arc::new(Mutex::new([0; 0xa0])),
            io_ports: Arc::new(Mutex::new([0; 0x4c])),
            iram: Box::new([0; 0x7f]),
            io_ie: Arc::new(AtomicU8::new(0)),
            rom: Box::new([0; 0x8000]),

            cycles: 0,
            pc: 0x0100, 
            sp: 0xfffe,
            regs: [0; 8],
            ime: false,
            halted: false,
            stopped: false,

            screen: Arc::new(Mutex::new([[0; 160]; 144])),
        }
    }

    /// Reads a byte from the specified address.
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7fff => {
                // TODO handle different ROM types. for now just treat it as simple 32kB ROM
                self.rom[addr as usize]
            },
            0x8000..=0x9fff => {
                let vram = self.vram.lock().unwrap();
                vram[(addr - 0x8000) as usize]
            },
            0xa000..=0xbfff => {
                panic!("TODO handle switchable RAM bank")
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
                panic!("Error: attempt to read from invalid memory")
            },
            0xff00..=0xff4b => {
                let io_ports = self.io_ports.lock().unwrap();
                io_ports[(addr - 0xff00) as usize]
            },
            0xff4c..=0xff7f => {
                panic!("Error: attempt to read from invalid memory")
            },
            0xff80..=0xfffe => {
                self.iram[(addr - 0xff80) as usize]
            },
            0xffff => {
                self.io_ie.load(Ordering::Relaxed)
            },
        }
    }

    /// Writes a byte to the specified address.
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x7fff => {
                // TODO handle different ROM types. for now just treat it as simple 32kB ROM
                panic!("Error: attempt to write to ROM")
            },
            0x8000..=0x9fff => {
                let mut vram = self.vram.lock().unwrap();
                vram[(addr - 0x8000) as usize] = value
            },
            0xa000..=0xbfff => {
                panic!("TODO handle switchable RAM bank")
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
                panic!("Error: attempt to read from invalid memory")
            },
            0xff00..=0xff4b => {
                let mut io_ports = self.io_ports.lock().unwrap();
                io_ports[(addr - 0xff00) as usize] = value
            },
            0xff4c..=0xff7f => {
                panic!("Error: attempt to read from invalid memory")
            },
            0xff80..=0xfffe => {
                self.iram[(addr - 0xff80) as usize] = value
            },
            0xffff => {
                self.io_ie.store(value, Ordering::Relaxed)
            },
        }
    }

    pub fn load_rom(&mut self, rom: &[u8; 0x8000]) {
        for i in 0..0x8000 {
            self.rom[i] = rom[i];
        }
    }
}

// TODO probably should move this somewhere, or make it a plain function
impl fmt::Display for Gameboy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f, 
            concat!(
                "PC: {:0>4X}, SP: {:0>X}, cycles: {:0>6}\n",
                "Z: {:2}, N: {:2}, H: {:2}, C: {:2}\n",
                "A: {:0>2X}, B: {:0>2X}, D: {:0>2X}, H: {:0>2X}\n",
                "F: {:0>2X}, C: {:0>2X}, E: {:0>2X}, L: {:0>2X}",
            ),
            self.pc, self.sp, self.cycles,
            (self.regs[RF] & FLAG_Z) >> 7, (self.regs[RF] & FLAG_N) >> 6,
            (self.regs[RF] & FLAG_H) >> 5, (self.regs[RF] & FLAG_C) >> 4,
            self.regs[RA], self.regs[RB], self.regs[RD], self.regs[RH],
            self.regs[RF], self.regs[RC], self.regs[RE], self.regs[RL],
        )
    }
}

/// Convert a register pair alias into a u16
pub fn rr_to_u16(gb: &Gameboy, reg_pair: RR) -> u16 {
    let upper = (gb.regs[reg_pair.0] as u16) << 8;
    let lower = gb.regs[reg_pair.1] as u16;
    upper ^ lower
}
