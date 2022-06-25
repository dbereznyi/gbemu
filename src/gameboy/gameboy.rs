use std::fmt;
use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicBool};

use crate::gameboy::cartridge::{*};

// Registers are accessed by indexing into Gameboy.regs
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

// LCDC settings
pub const LCDC_ON: u8           = 0b1000_0000;
pub const LCDC_WIN_TILE_MAP: u8 = 0b0100_0000;
pub const LCDC_WIN_DISP: u8     = 0b0010_0000;
pub const LCDC_TILE_DATA: u8    = 0b0001_0000;
pub const LCDC_BG_TILE_MAP: u8  = 0b0000_1000;
pub const LCDC_OBJ_SIZE: u8     = 0b0000_0100;
pub const LCDC_OBJ_DISP: u8     = 0b0000_0010;
pub const LCDC_BG_DISP: u8      = 0b0000_0001;

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
pub const VBLANK: u8    = 0b0000_0001;
pub const LCDC: u8      = 0b0000_0010;
pub const TIMER: u8     = 0b0000_0100;
pub const SERIAL: u8    = 0b0000_1000;
pub const HI_TO_LOW: u8 = 0b0001_0000;

pub struct Gameboy {
    /// Working RAM, accessible by CPU only.
    wram: Box<[u8; 0x2000]>,
    /// Video RAM, accessible by CPU and PPU (but not at the same time).
    pub vram: Arc<Mutex<[u8; 0x2000]>>,
    /// Object (sprite) Attribute Memory, used by PPU and can be written to via DMA transfer.
    pub oam: Arc<Mutex<[u8; 0xa0]>>,
    /// IO Ports and hardware control registers.
    pub io_ports: Arc<Mutex<[u8; 0x4d]>>,
    /// Internal RAM, used e.g. for stack.
    iram: Box<[u8; 0x7f]>,
    /// The currently-loaded cartridge.
    cartridge: Cartridge,

    /// Elapsed machine cycles.
    pub cycles: u64, 
    pub pc: u16,
    pub sp: u16,
    /// Registers A, B, C, D, E, F, H, L.
    pub regs: [u8; 8], 
    /// Interrupt Master Enable.
    pub ime: Arc<AtomicBool>,
    pub halted: Arc<AtomicBool>,
    pub stopped: Arc<AtomicBool>,

    /// CPU can wait on this variable to sleep until interrupted.
    pub interrupt_received: Arc<(Mutex<bool>, Condvar)>,

    /// Holds pixel data to be drawn to the screen.
    pub screen: Arc<Mutex<[[u8; 160]; 144]>>,
}

impl Gameboy {
    pub fn new(cartridge: Cartridge) -> Gameboy {
        Gameboy {
            wram: Box::new([0; 0x2000]),
            vram: Arc::new(Mutex::new([0; 0x2000])),
            oam: Arc::new(Mutex::new([0; 0xa0])),
            io_ports: Arc::new(Mutex::new([0; 0x4d])),
            iram: Box::new([0; 0x7f]),
            cartridge: cartridge,

            cycles: 0,
            pc: 0x0100, 
            sp: 0xfffe,
            regs: [0; 8],
            ime: Arc::new(AtomicBool::new(false)),
            halted: Arc::new(AtomicBool::new(false)),
            stopped: Arc::new(AtomicBool::new(false)),

            interrupt_received: Arc::new((Mutex::new(false), Condvar::new())),

            screen: Arc::new(Mutex::new([[0; 160]; 144])),
        }
    }

    /// Reads a byte from the specified address according to the memory map.
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
                let io_ports = self.io_ports.lock().unwrap();
                io_ports[IO_IE]
            },
        }
    }

    /// Writes a byte to the specified address according to the memory map.
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
                let mut io_ports = self.io_ports.lock().unwrap();
                io_ports[IO_IE] = value
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

/// Converts a register pair alias into a u16.
pub fn rr_to_u16(gb: &Gameboy, reg_pair: RR) -> u16 {
    let upper = (gb.regs[reg_pair.0] as u16) << 8;
    let lower = gb.regs[reg_pair.1] as u16;
    upper ^ lower
}
