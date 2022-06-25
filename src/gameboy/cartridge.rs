use std::sync::{Arc};
use std::sync::atomic::{AtomicU8, Ordering};

/// A Gameboy cartridge with ROM and potentially RAM that can be read/written to.
/// Cartridges with memory bank controllers use writes to ROM to do things like select ROM banks.
pub trait CartridgeT {
    fn read_rom(&self, addr: u16) -> u8;
    fn read_ram(&self, addr: u16) -> u8;
    fn write_rom(&mut self, addr: u16, value: u8);
    fn write_ram(&mut self, addr: u16, value: u8);
}

pub type Cartridge = Box<dyn CartridgeT + Send>;

#[derive(Debug)]
pub enum CartridgeLoadErr {
    InvalidCartridgeType(u8),
    UnsupportedCartridgeType(u8),
}

enum MbcType {
    NoMbc,
    Mbc1,
    Mbc2,
    Mbc3,
    Mbc5,
}

pub fn load_cartridge(bytes: &[u8]) -> Result<Cartridge, CartridgeLoadErr> {
    let cartridge_type = bytes[0x147];
    let (mbc_type, has_battery) = match cartridge_type {
        0x00 => Ok((MbcType::NoMbc, false)),
        0x01 => Ok((MbcType::Mbc1, false)),
        0x02 => Ok((MbcType::Mbc1, false)),
        0x03 => Ok((MbcType::Mbc1, true)),
        0x04 => Err(CartridgeLoadErr::InvalidCartridgeType(cartridge_type)),
        0x05 => Ok((MbcType::Mbc2, false)),
        0x06 => Ok((MbcType::Mbc2, true)),
        0x07 => Err(CartridgeLoadErr::InvalidCartridgeType(cartridge_type)),
        0x08 => Ok((MbcType::NoMbc, false)),
        0x09 => Ok((MbcType::NoMbc, true)),
        0x0a => Err(CartridgeLoadErr::InvalidCartridgeType(cartridge_type)),
        // Currently don't support MMM01
        0x0b..=0x0e => Err(CartridgeLoadErr::UnsupportedCartridgeType(cartridge_type)),
        0x0f => Ok((MbcType::Mbc3, true)),
        0x10 => Ok((MbcType::Mbc3, true)), // Unlike 0x13, this has a built-in timer
        0x11 => Ok((MbcType::Mbc3, false)),
        0x12 => Ok((MbcType::Mbc3, false)),
        0x13 => Ok((MbcType::Mbc3, true)),
        0x14..=0x18 => Err(CartridgeLoadErr::InvalidCartridgeType(cartridge_type)),
        0x19 => Ok((MbcType::Mbc5, false)),
        0x1a => Ok((MbcType::Mbc5, false)),
        0x1b => Ok((MbcType::Mbc5, true)),
        0x1c => Ok((MbcType::Mbc5, false)), // Unlike 0x19, this also has rumble
        0x1d => Ok((MbcType::Mbc5, false)), // Unlike 0x1a, this also has rumble+SRAM 
        0x1e => Ok((MbcType::Mbc5, true)), // Unlike 0x1b, this also has rumble+SRAM
        // Currently don't support Pocket Camera
        0x1f => Err(CartridgeLoadErr::UnsupportedCartridgeType(cartridge_type)),
        0x20..=0xfc => Err(CartridgeLoadErr::InvalidCartridgeType(cartridge_type)),
        // Currently don't support Hudson HuC-3 or Hudson HuC-1
        0xfd..=0xfe => Err(CartridgeLoadErr::UnsupportedCartridgeType(cartridge_type)),
        0xff => Err(CartridgeLoadErr::InvalidCartridgeType(cartridge_type)),
    }?;

    let has_timer = cartridge_type == 0x0f || cartridge_type == 0x10;

    match mbc_type {
        MbcType::NoMbc => Ok(Box::new(CartridgeNoMbc::new(bytes, has_battery))),
        MbcType::Mbc1 => Ok(Box::new(CartridgeMbc1::new(bytes, has_battery))),
        MbcType::Mbc2 => Ok(Box::new(CartridgeMbc2::new(bytes, has_battery))),
        MbcType::Mbc3 => Ok(Box::new(CartridgeMbc3::new(bytes, has_battery, has_timer))),
        _ => panic!("TODO implement"),
    }
}

/// A cartridge with no memory bank controller.
struct CartridgeNoMbc {
    /// Two banks of 0x4000 bytes each, always mapped to 0x0000-0x7fff.
    rom: Box<[u8; 0x8000]>,
    /// A single bank of 0x2000 bytes that is always mapped to 0xa000-0xbfff.
    ram: Box<[u8; 0x2000]>,
}

impl CartridgeNoMbc {
    fn new(bytes: &[u8], has_battery: bool) -> Self {
        let mut rom = Box::new([0; 0x8000]);
        for (i, byte) in bytes.iter().enumerate() {
            rom[i] = *byte;
        }

        // TODO load RAM from file if has_battery is true
        let ram = Box::new([0; 0x2000]);

        Self {
            rom: rom,
            ram: ram,
        }
    }
}

impl CartridgeT for CartridgeNoMbc {
    fn read_rom(&self, addr: u16) -> u8 {
        self.rom[addr as usize]
    }

    fn read_ram(&self, addr: u16) -> u8 {
        self.ram[addr as usize - 0xa000]
    }

    fn write_rom(&mut self, addr: u16, value: u8) {
        panic!("Cannot write to ROM in no-mbc cartridge")
    }

    fn write_ram(&mut self, addr: u16, value: u8) {
        self.ram[addr as usize - 0xa000] = value
    }
}

/// A cartridge with an Mbc1-type memory bank controller.
struct CartridgeMbc1 {
    /// Whether or not RAM is currently write-protected.
    write_protect_on: bool,
    /// ROM bank select. Can take values between 0x01-0x1f (banks #1-32).
    /// In 16Mbit ROM / 8Kbyte RAM mode, selects an individual bank from a range of banks.
    rom_bank_code: u8,
    /// In 16Mbit ROM / 8KByte RAM mode: 
    ///     Upper ROM bank select. Can take values between 0x00-0x03, to select banks 0x01-1f,
    ///     0x21-0x3f, 0x41-0x5f, 0x61-0x7f, respectively.
    /// In 4Mbit ROM / 32KByte RAM mode:
    ///     Which RAM bank is currently selected. Can take values between 0x00-0x03 (banks #0-3).
    ram_or_upper_rom_bank_code: u8,
    /// When set to true, 4Mbit ROM / 32KByte RAM mode is enabled.
    /// When set to false, 16Mbit ROM / 8KByte RAM mode is enabled.
    /// This flag determines whether ram_or_upper_rom_bank_code selects RAM or ROM banks.  
    /// Defaults to false.
    large_ram_mode: bool,
    /// 16Mbit ROM data (up to 128 banks of 0x4000 bytes each).
    rom: Box<[u8; 128 * 0x4000]>,
    /// 256Kbit RAM data (up to 4 banks of 0x2000 bytes each).
    ram: Box<[u8; 4 * 0x2000]>,
}

impl CartridgeMbc1 {
    fn new(bytes: &[u8], has_battery: bool) -> Self {
        let mut rom = Box::new([0; 128 * 0x4000]);
        for (i, byte) in bytes.iter().enumerate() {
            rom[i] = *byte;
        }

        // TODO create/open a file to persist cartridge RAM if has_battery is true

        Self {
            write_protect_on: true,
            rom_bank_code: 0x01,
            ram_or_upper_rom_bank_code: 0x00,
            large_ram_mode: false,
            rom: rom,
            ram: Box::new([0; 4 * 0x2000]),
        }
    }
}

impl CartridgeT for CartridgeMbc1 {
    fn read_rom(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x3fff => self.rom[addr as usize],
            0x4000..=0x7fff => {
                let base_addr = 
                    ((self.ram_or_upper_rom_bank_code as usize * 0x20) + self.rom_bank_code as usize)
                    * 0x4000;
                self.rom[base_addr + addr as usize - 0x4000]
            },
            _ => panic!("Invalid address {:0>4X} for ROM read", addr),
        }
    }

    fn read_ram(&self, addr: u16) -> u8 {
        if !self.write_protect_on {
            let base_addr = self.ram_or_upper_rom_bank_code as usize * 0x2000;
            self.ram[base_addr + addr as usize - 0xa000]
        } else {
            0xff
        }
    }

    fn write_rom(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1fff => {
                self.write_protect_on = value != 0xa0;
            },
            0x2000..=0x3fff => {
                self.rom_bank_code = if value == 0x00 { 0x01 } else { value };
            },
            0x4000..=0x5fff => {
                self.ram_or_upper_rom_bank_code = value & 0b11;
            },
            0x6000..=0x7fff => {
                if value & 1 > 0 {
                    self.large_ram_mode = true;
                } else {
                    self.large_ram_mode = false;
                }
            },
            _ => panic!("Invalid address {:0>4X} for ROM write", addr),
        }
    }

    fn write_ram(&mut self, addr: u16, value: u8) {
        if !self.write_protect_on {
            let base_addr = self.ram_or_upper_rom_bank_code as usize * 0x2000;
            self.ram[base_addr + addr as usize - 0xa000] = value;
        } else {
            // Will probably replace this with a no-op. For now useful for debugging
            panic!("Attempt to write to RAM while write protect still enabled")
        }
    }
}

/// A cartridge with an Mbc2-type memory bank controller.
struct CartridgeMbc2 {
    /// Whether or not RAM is currently write-protected.
    write_protect_on: bool,
    /// ROM bank select. Can take values between 0x01-0x0f (banks #1-15).
    rom_bank_code: u8,
    /// 16 banks of 0x4000 bytes each.
    rom: Box<[u8; 16 * 0x4000]>,
    /// Mapped to 0xa000-0xa1ff, but only bottom 4 bits of each address are useable.
    ram: Box<[u8; 512]>,
}

impl CartridgeMbc2 {
    fn new(bytes: &[u8], has_battery: bool) -> Self {
        let mut rom = Box::new([0; 16 * 0x4000]);
        for (i, byte) in bytes.iter().enumerate() {
            rom[i] = *byte;
        }

        // TODO create/open a file to persist cartridge RAM if has_battery is true

        Self {
            write_protect_on: true,
            rom_bank_code: 0x00,
            rom: rom,
            ram: Box::new([0; 512]),
        }
    }
}

impl CartridgeT for CartridgeMbc2 {
    fn read_rom(&self, addr: u16) -> u8 {
        if addr < 0x4000 {
            self.rom[addr as usize]
        } else {
            let base_addr = self.rom_bank_code as usize * 0x4000;
            self.rom[base_addr + addr as usize - 0x4000]
        }
    }

    fn read_ram(&self, addr: u16) -> u8 {
        if !self.write_protect_on {
            self.ram[addr as usize - 0xa000] & 0b0000_1111
        } else {
            0xff
        }
    }

    fn write_rom(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x0fff => self.write_protect_on = value != 0x0a,
            0x2100..=0x21ff => self.rom_bank_code = value & 0b0000_1111,
            _ => panic!("Invalid address {:0>4X} for ROM write", addr),
        }
    }

    fn write_ram(&mut self, addr: u16, value: u8) {
        if !self.write_protect_on {
            self.ram[addr as usize - 0xa000] = value & 0b0000_1111;
        } else {
            // Will probably replace this with a no-op. For now useful for debugging
            panic!("Attempt to write to RAM while write protect still enabled")
        }
    }
}

struct CartridgeMbc3 {
    /// Whether or not RAM is currently write-protected.
    write_protect_on: bool,
    /// ROM bank select. Can take values between 0x01-0x7f (banks #1-128).
    rom_bank_code: u8,
    /// RAM bank select. Can take values between 0x00-0x03 (banks #0-3) or 0x08-0x0c (RTC
    /// registers).
    ram_bank_code: u8,
    /// 128 banks of 0x4000 bytes each.
    rom: Box<[u8; 128 * 0x4000]>,
    /// 4 banks of 0x2000 bytes each.
    ram: Box<[u8; 4 * 0x2000]>,
    /// RTC register: seconds counter.
    rtc_s: Arc<AtomicU8>,
    /// RTC register: minutes counter.
    rtc_m: Arc<AtomicU8>,
    /// RTC register: hours counter.
    rtc_h: Arc<AtomicU8>,
    /// RTC register: days (lower) counter.
    rtc_dl: Arc<AtomicU8>,
    /// RTC register: days (higher) counter.
    rtc_dh: Arc<AtomicU8>,
}

impl CartridgeMbc3 {
    fn new(bytes: &[u8], has_battery: bool, has_timer: bool) -> Self {
        let mut rom = Box::new([0; 128 * 0x4000]);
        for (i, byte) in bytes.iter().enumerate() {
            rom[i] = *byte;
        }

        // TODO read RAM from file is battery-powered
        let ram = Box::new([0; 4 * 0x2000]);

        // TODO read these from file if battery-powered and has timer
        let rtc_s = 0x00;
        let rtc_m = 0x00;
        let rtc_h = 0x00;
        let rtc_dl = 0x00;
        let rtc_dh = 0x00;

        Self {
            write_protect_on: true,
            rom_bank_code: 0x01,
            ram_bank_code: 0x00,
            rom: rom,
            ram: ram,
            rtc_s: Arc::new(AtomicU8::new(rtc_s)),
            rtc_m: Arc::new(AtomicU8::new(rtc_m)),
            rtc_h: Arc::new(AtomicU8::new(rtc_h)),
            rtc_dl: Arc::new(AtomicU8::new(rtc_dl)),
            rtc_dh: Arc::new(AtomicU8::new(rtc_dh)),
        }
    }
}

impl CartridgeT for CartridgeMbc3 {
    fn read_rom(&self, addr: u16) -> u8 {
        if addr < 0x4000 {
            self.rom[addr as usize]
        } else {
            let base_addr = (self.rom_bank_code as usize) * 0x4000;
            self.rom[base_addr + addr as usize]
        }
    }

    fn read_ram(&self, addr: u16) -> u8 {
        if !self.write_protect_on {
            match self.ram_bank_code {
                0x00..=0x03 => {
                    let base_addr = (self.ram_bank_code as usize) * 0x2000;
                    self.ram[base_addr + addr as usize]
                },
                0x08 => self.rtc_s.load(Ordering::Relaxed),
                0x09 => self.rtc_m.load(Ordering::Relaxed),
                0x0a => self.rtc_h.load(Ordering::Relaxed),
                0x0b => self.rtc_dl.load(Ordering::Relaxed),
                0x0c => self.rtc_dh.load(Ordering::Relaxed),
                _ => panic!("Invalid RAM bank code {}", self.ram_bank_code),
            }
        } else {
            0xff
        }
    }

    fn write_rom(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1fff => self.write_protect_on = value != 0x0a,
            0x2000..=0x3fff => self.rom_bank_code = value & 0b0111_1111,
            0x4000..=0x5fff => self.ram_bank_code = value,
            _ => panic!("Invalid address {:0>4X} for ROM write", addr),
        }
    }

    fn write_ram(&mut self, addr: u16, value: u8) {
        if !self.write_protect_on {
            match self.ram_bank_code {
                0x00..=0x03 => {
                    let base_addr = (self.ram_bank_code as usize) * 0x2000;
                    self.ram[base_addr + addr as usize] = value
                },
                0x08 => self.rtc_s.store(value, Ordering::Relaxed),
                0x09 => self.rtc_m.store(value, Ordering::Relaxed),
                0x0a => self.rtc_h.store(value, Ordering::Relaxed),
                0x0b => self.rtc_dl.store(value, Ordering::Relaxed),
                0x0c => self.rtc_dh.store(value, Ordering::Relaxed),
                _ => panic!("Invalid RAM bank code {}", self.ram_bank_code),
            }
        } else {
            // Will probably replace this with a no-op. For now useful for debugging
            panic!("Attempt to write to RAM while write protect still enabled")
        }
    }
}
