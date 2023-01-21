use std::convert::TryInto;
use std::str;

fn read_u8(bytes: &[u8], i: &mut usize) -> u8 {
    let value = bytes[*i];
    *i += 1;
    value
}

fn read_u16(bytes: &[u8], i: &mut usize) -> u16 {
    let value = u16::from_le_bytes(bytes[*i..*i+2].try_into().unwrap());
    *i += 2;
    value
}

fn read_u32(bytes: &[u8], i: &mut usize) -> u32 {
    let value = u32::from_le_bytes(bytes[*i..*i+4].try_into().unwrap());
    *i += 4;
    value
}

fn read_u64(bytes: &[u8], i: &mut usize) -> u64 {
    let value = u64::from_le_bytes(bytes[*i..*i+8].try_into().unwrap());
    *i += 8;
    value
}

fn read_memory_range<'a>(bytes: &'a [u8], i: &mut usize) -> Result<&'a [u8], String> {
    let size = read_u32(bytes, i) as usize;
    let start = read_u32(bytes, i) as usize;
    if start + size > bytes.len() {
        return Err(format!("Invalid memory range: start+size points beyond file end (start: {}, size: {}, file length: {})", start, size, bytes.len()));
    }
    Ok(&bytes[start..start+size])
}

#[derive(Debug)]
pub struct NameBlock {
    pub name: Vec<u8>,
}

impl NameBlock {
    fn read(bytes: &[u8], i: &mut usize) -> Result<Self, String> {
        if bytes[*i..].len() < 5 {
            return Err("NAME block: Not enough bytes".to_string());
        }
        let identifier: &[u8] = bytes[*i..*i+4].try_into().unwrap();
        if identifier != b"NAME" {
            return Err("NAME block: Bad identifier".to_string());
        }
        *i += 4;
        let length = read_u8(bytes, i) as usize;
        if bytes[*i..].len() < length {
            return Err("NAME block: Bad length".to_string());
        }
        
        let block = Self {
            name: bytes[*i..*i+length].to_vec(),
        };
        *i += length;
        Ok(block)
    }
}

#[derive(Debug)]
pub struct InfoBlock {
    pub title: [u8; 16],
    pub checksum: u16,
}

impl InfoBlock {
    fn read(bytes: &[u8], i: &mut usize) -> Result<Self, String> {
        if bytes[*i..].len() < 5+0x12 {
            return Err("INFO block: Not enough bytes".to_string());
        }
        let identifier: &[u8] = bytes[*i..*i+4].try_into().unwrap();
        if identifier != b"INFO" {
            return Err("INFO block: Bad identifier".to_string());
        }
        *i += 4;
        let length = read_u8(bytes, i);
        if length != 0x12 {
            return Err("INFO block: Bad length".to_string());
        }

        let title: [u8; 16] = bytes[*i..*i+16].try_into().unwrap();
        *i += 16;
        let checksum = read_u16(bytes, i);
        let block = Self {
            title: title,
            checksum: checksum,
        };
        Ok(block)
    }
}

#[derive(Debug)]
pub struct CoreBlock<'a> {
    pub major: u16,
    pub minor: u16,
    pub model: [u8; 4],
    pub pc: u16,
    pub af: u16,
    pub bc: u16,
    pub de: u16,
    pub hl: u16,
    pub sp: u16,
    pub ime: u8, // 0 or 1
    pub ie: u8,
    pub execution_state: u8, // 0 = running, 1 = halted, 2 = stopped
    pub reserved: u8, // Must be 0
    pub memory_mapped_registers: &'a [u8; 128],
    pub ram: &'a [u8],
    pub vram: &'a [u8],
    pub mbc_ram: &'a [u8],
    pub oam: &'a [u8],
    pub hram: &'a [u8],
    pub bgp: &'a [u8],
    pub obp: &'a [u8],
}

impl<'a> CoreBlock<'a> {
    fn read(bytes: &'a [u8], i: &mut usize) -> Result<Self, String> {
        if bytes[*i..].len() < 5+0xd0 {
            return Err("CORE block: Not enough bytes".to_string());
        }
        let identifier: &[u8] = bytes[*i..*i+4].try_into().unwrap();
        if identifier != b"CORE" {
            return Err("CORE block: Bad identifier".to_string());
        }
        *i += 4;
        let length = read_u8(bytes, i);
        if length != 0xd0 {
            return Err("CORE block: Bad length".to_string());
        }

        let major = read_u16(bytes, i);
        let minor = read_u16(bytes, i);
        let model: [u8; 4] = bytes[*i..*i+4].try_into().unwrap();
        *i += 4;
        let pc = read_u16(bytes, i);
        let af = read_u16(bytes, i);
        let bc = read_u16(bytes, i);
        let de = read_u16(bytes, i);
        let hl = read_u16(bytes, i);
        let sp = read_u16(bytes, i);
        let ime = read_u8(bytes, i);
        let ie = read_u8(bytes, i);
        let execution_state = read_u8(bytes, i);
        let reserved = read_u8(bytes, i);

        let memory_mapped_registers: &[u8; 128] = bytes[*i..*i+128].try_into().unwrap();
        *i += 128;

        let ram = read_memory_range(bytes, i)?;
        let vram = read_memory_range(bytes, i)?;
        let mbc_ram = read_memory_range(bytes, i)?;
        let oam = read_memory_range(bytes, i)?;
        let hram = read_memory_range(bytes, i)?;
        let bgp = read_memory_range(bytes, i)?;
        let obp = read_memory_range(bytes, i)?;

        let block = Self {
            major,
            minor,
            model,
            pc,
            af,
            bc,
            de,
            hl,
            sp,
            ime,
            ie,
            execution_state,
            reserved,
            memory_mapped_registers,
            ram,
            vram,
            mbc_ram,
            oam,
            hram,
            bgp,
            obp,
        };

        Ok(block)
    }
}

#[derive(Debug)]
pub struct XaomBlock {
    pub bytes: [u8; 0x60],
}

impl XaomBlock {
    fn read(bytes: &[u8], i: &mut usize) -> Result<Self, String> {
        if bytes[*i..].len() < 5+0x60 {
            return Err("XAOM block: Not enough bytes".to_string());
        }
        let identifier: &[u8] = bytes[*i..*i+4].try_into().unwrap();
        if identifier != b"XAOM" {
            return Err("XAOM block: Bad identifier".to_string());
        }
        *i += 4;
        let length = read_u8(bytes, i) as usize;
        if length != 0x60 {
            return Err("XAOM block: Bad length".to_string());
        }

        let xaom_bytes = bytes[*i..*i+0x60].try_into().unwrap();
        *i += 0x60;
        let block = Self {
            bytes: xaom_bytes,
        };
        Ok(block)
    }
}

#[derive(Debug)]
pub struct MbcBlock {
    pub registers: Vec<(u16, u8)>,
}

impl MbcBlock {
    fn read(bytes: &[u8], i: &mut usize) -> Result<Self, String> {
        if bytes[*i..].len() < 5+3 {
            return Err("MBC block: Not enough bytes".to_string());
        }
        let identifier: &[u8] = bytes[*i..*i+4].try_into().unwrap();
        if identifier != b"MBC " {
            return Err("MBC block: Bad identifier".to_string());
        }
        *i += 4;
        let length = read_u8(bytes, i) as usize;
        if length % 3 != 0 || length > bytes[*i..].len() {
            return Err("MBC block: Bad length".to_string());
        }

        let mut registers = Vec::with_capacity(length / 3);
        for _ in 0..length/3 {
            let addr = read_u16(bytes, i);
            let byte = read_u8(bytes, i);
            registers.push((addr, byte));
        }
        let block = Self {
            registers
        };
        Ok(block)
    }
}

#[derive(Debug)]
pub struct RtcBlock {
    pub seconds: u8,
    pub minutes: u8,
    pub hours: u8,
    pub days_low: u8,
    pub days_high: u8,
    pub latched_seconds: u8,
    pub latched_minutes: u8,
    pub latched_hours: u8,
    pub latched_days_low: u8,
    pub latched_days_high: u8,
    pub unix_timestamp: u64,
}

impl RtcBlock {
    fn read(bytes: &[u8], i: &mut usize) -> Result<Self, String> {
        if bytes[*i..].len() < 5+0x30 {
            return Err("RTC block: Not enough bytes".to_string());
        }
        let identifier: &[u8] = bytes[*i..*i+4].try_into().unwrap();
        if identifier != b"RTC " {
            return Err("RTC block: Bad identifier".to_string());
        }
        *i += 4;
        let length = read_u8(bytes, i) as usize;
        if length != 0x30 {
            return Err("RTC block: Bad length".to_string());
        }

        // Don't use read_u8 here because of required 3-byte padding.
        let seconds = bytes[*i];
        *i += 4;
        let minutes = bytes[*i];
        *i += 4;
        let hours = bytes[*i];
        *i += 4;
        let days_low = bytes[*i];
        *i += 4;
        let days_high = bytes[*i];
        *i += 4;
        let latched_seconds = bytes[*i];
        *i += 4;
        let latched_minutes = bytes[*i];
        *i += 4;
        let latched_hours = bytes[*i];
        *i += 4;
        let latched_days_low = bytes[*i];
        *i += 4;
        let latched_days_high = bytes[*i];
        *i += 4;
        let unix_timestamp = read_u64(bytes, i);
        let block = Self {
            seconds,
            minutes,
            hours,
            days_low,
            days_high,
            latched_seconds,
            latched_minutes,
            latched_hours,
            latched_days_low,
            latched_days_high,
            unix_timestamp,
        };
        Ok(block)
    }
}

#[derive(Debug)]
pub struct Huc3Block {
    pub unix_timestamp: u64,
    pub rtc_minutes: u16,
    pub rtc_days: u16,
    pub alarm_minutes: u16,
    pub alarm_days: u16,
    pub alarm_enabled: u8, // 0 or 1
}

impl Huc3Block {
    fn read(bytes: &[u8], i: &mut usize) -> Result<Self, String> {
        if bytes[*i..].len() < 5+0x11 {
            return Err("HUC3 block: Not enough bytes".to_string());
        }
        let identifier: &[u8] = bytes[*i..*i+4].try_into().unwrap();
        if identifier != b"HUC3" {
            return Err("HUC3 block: Bad identifier".to_string());
        }
        *i += 4;
        let length = read_u8(bytes, i) as usize;
        if length != 0x11 {
            return Err("HUC3 block: Bad length".to_string());
        }
        
        let block = Self {
            unix_timestamp: read_u64(bytes, i),
            rtc_minutes: read_u16(bytes, i),
            rtc_days: read_u16(bytes, i),
            alarm_minutes: read_u16(bytes, i),
            alarm_days: read_u16(bytes, i),
            alarm_enabled: read_u8(bytes, i),
        };
        if block.alarm_enabled > 1 {
            return Err("HUC3 block: Bad value for alarm_enabled".to_string());
        }
        Ok(block)
    }
}

#[derive(Debug)]
pub struct Tpp1Block {
    pub unix_timestamp: u64,
    pub rtc: u32,
    pub latched_rtc: u32,
    pub mr4: u8,
}

impl Tpp1Block {
    fn read(bytes: &[u8], i: &mut usize) -> Result<Self, String> {
        if bytes[*i..].len() < 5+0x11 {
            return Err("TPP1 block: Not enough bytes".to_string());
        }
        let identifier: &[u8] = bytes[*i..*i+4].try_into().unwrap();
        if identifier != b"TPP1" {
            return Err("TPP1 block: Bad identifier".to_string());
        }
        *i += 4;
        let length = read_u8(bytes, i) as usize;
        if length != 0x11 {
            return Err("TPP1 block: Bad length".to_string());
        }

        let block = Self {
            unix_timestamp: read_u64(bytes, i),
            rtc: read_u32(bytes, i),
            latched_rtc: read_u32(bytes, i),
            mr4: read_u8(bytes, i),
        };
        Ok(block)
    }
}

#[derive(Debug)]
pub struct Mbc7Block {
    pub flags: u8,
    pub argument_bits_left: u8,
    pub eeprom_command: u16,
    pub pending_bits: u16,
    pub latched_gyro_x: u16,
    pub latched_gyro_y: u16,
}

impl Mbc7Block {
    fn read(bytes: &[u8], i: &mut usize) -> Result<Self, String> {
        if bytes[*i..].len() < 5+0x0a {
            return Err("MBC7 block: Not enough bytes".to_string());
        }
        let identifier: &[u8] = bytes[*i..*i+4].try_into().unwrap();
        if identifier != b"MBC7" {
            return Err("MBC7 block: Bad identifier".to_string());
        }
        *i += 4;
        let length = read_u8(bytes, i);
        if length != 0x0a {
            return Err("MBC7 block: Bad length".to_string());
        }

        let block = Self {
            flags: read_u8(bytes, i),
            argument_bits_left: read_u8(bytes, i),
            eeprom_command: read_u16(bytes, i),
            pending_bits: read_u16(bytes, i),
            latched_gyro_x: read_u16(bytes, i),
            latched_gyro_y: read_u16(bytes, i),
        };
        Ok(block)
    }
}

#[derive(Debug)]
pub struct SgbBlock<'a> {
    pub border_tile_data: &'a [u8], // 0x2000
    pub border_tilemap: &'a [u8], // 0x800
    pub border_palettes: &'a [u8], // 0x80
    pub active_colorization_palettes: &'a [u8], // 0x20
    pub ram_colorization_palettes: &'a [u8], // 0x1000
    pub attribute_map: &'a [u8], // 0x168
    pub attribute_files: &'a [u8], // 0xfd2
    pub multiplayer_status: u8, // High nibble = player count (1, 2 or 4),
                                // Low nibble = current player (with player 1 = 0)
}

impl<'a> SgbBlock<'a> {
    fn read(bytes: &'a [u8], i: &mut usize) -> Result<Self, String> {
        if bytes[*i..].len() < 5+0x39 {
            return Err("SGB block: Not enough bytes".to_string());
        }
        let identifier: &[u8] = bytes[*i..*i+4].try_into().unwrap();
        if identifier != b"SGB " {
            return Err("SGB block: Bad identifier".to_string());
        }
        *i += 4;
        let length = read_u8(bytes, i) as usize;
        if length != 0x39 {
            return Err("SGB block: Bad length".to_string());
        }

        let block = Self {
            border_tile_data: read_memory_range(bytes, i)?,
            border_tilemap: read_memory_range(bytes, i)?,
            border_palettes: read_memory_range(bytes, i)?,
            active_colorization_palettes: read_memory_range(bytes, i)?,
            ram_colorization_palettes: read_memory_range(bytes, i)?,
            attribute_map: read_memory_range(bytes, i)?,
            attribute_files: read_memory_range(bytes, i)?,
            multiplayer_status: read_u8(bytes, i),
        };
        Ok(block)
    }
}

#[derive(Debug)]
pub struct Bess<'a> {
    pub filename: String,
    pub name_block: Option<NameBlock>,
    pub info_block: InfoBlock,
    pub core_block: CoreBlock<'a>,
    pub xaom_block: Option<XaomBlock>,
    pub mbc_block: Option<MbcBlock>,
    pub rtc_block: Option<RtcBlock>,
    pub huc3_block: Option<Huc3Block>,
    pub tpp1_block: Option<Tpp1Block>,
    pub mbc7_block: Option<Mbc7Block>,
    pub sgb_block: Option<SgbBlock<'a>>,
}

impl<'a> Bess<'a> {
    pub fn new(bytes: &'a [u8], filename: &str) -> Result<Bess<'a>, String> {
        if bytes.len() < 8 {
            return Err("File too short".to_string());
        }
        let first_block_offset = bytes[bytes.len() - 8];
        let bess_string: &[u8] = bytes[bytes.len() - 4 ..].try_into().unwrap();
        if bess_string != b"BESS" {
            return Err("Missing 'BESS' string at end of file".to_string());
        }
        
        let mut name_block: Option<NameBlock> = None;
        let mut info_block: Option<InfoBlock> = None;
        let mut core_block: Option<CoreBlock> = None;
        let mut xaom_block: Option<XaomBlock> = None;
        let mut mbc_block: Option<MbcBlock> = None;
        let mut rtc_block: Option<RtcBlock> = None;
        let mut huc3_block: Option<Huc3Block> = None;
        let mut tpp1_block: Option<Tpp1Block> = None;
        let mut mbc7_block: Option<Mbc7Block> = None;
        let mut sgb_block: Option<SgbBlock> = None;
        let mut has_end_block = false;

        let mut i = first_block_offset as usize;
        while i + 4 < bytes.len() {
            let identifier: &[u8] = bytes[i..i+4].try_into().unwrap();
            match identifier {
                b"NAME" => match NameBlock::read(&bytes, &mut i) {
                    Err(err) => { eprintln!("{}", err); },
                    Ok(block) => { name_block = Some(block); }
                },
                b"INFO" => match InfoBlock::read(&bytes, &mut i) {
                    Err(err) => { return Err(err); },
                    Ok(block) => { info_block = Some(block); }
                },
                b"CORE" => match CoreBlock::read(&bytes, &mut i) {
                    Err(err) => { return Err(err); },
                    Ok(block) => { core_block = Some(block); }
                },
                b"XAOM" => match XaomBlock::read(&bytes, &mut i) {
                    Err(err) => { eprintln!("{}", err); },
                    Ok(block) => { xaom_block = Some(block); }
                },
                b"MBC " => match MbcBlock::read(&bytes, &mut i) {
                    Err(err) => { eprintln!("{}", err); },
                    Ok(block) => { mbc_block = Some(block); }
                },
                b"RTC " => match RtcBlock::read(&bytes, &mut i) {
                    Err(err) => { eprintln!("{}", err); },
                    Ok(block) => { rtc_block = Some(block); }
                },
                b"HUC3" => match Huc3Block::read(&bytes, &mut i) {
                    Err(err) => { eprintln!("{}", err); },
                    Ok(block) => { huc3_block = Some(block); }
                },
                b"TPP1" => match Tpp1Block::read(&bytes, &mut i) {
                    Err(err) => { eprintln!("{}", err); },
                    Ok(block) => { tpp1_block = Some(block); }
                },
                b"MBC7" => match Mbc7Block::read(&bytes, &mut i) {
                    Err(err) => { eprintln!("{}", err); },
                    Ok(block) => { mbc7_block = Some(block); }
                },
                b"SGB " => match SgbBlock::read(&bytes, &mut i) {
                    Err(err) => { eprintln!("{}", err); },
                    Ok(block) => { sgb_block = Some(block); }
                },
                b"END " => {
                    has_end_block = true;
                    break;
                },
                _ => { return Err(format!("Invalid identifier {:?}", identifier)); },
            }
        }

        if !has_end_block {
            return Err("Missing END block".to_string());
        }

        if info_block.is_none() {
            return Err("Missing INFO block".to_string());
        }
        let info_block = info_block.unwrap();

        if core_block.is_none() {
            return Err("Missing CORE block".to_string());
        }
        let core_block = core_block.unwrap();

        let bess = Self {
            filename: filename.to_string(),
            name_block,
            info_block,
            core_block,
            xaom_block,
            mbc_block,
            rtc_block,
            huc3_block,
            tpp1_block,
            mbc7_block,
            sgb_block,
        };

        Ok(bess)
    }
}

#[cfg(test)]
mod tests {
    use super::{*};

    #[test]
    fn name_block_read() {
        let mut bytes = b"NAME".to_vec();
        bytes.push(10);
        for b in b"0123456789" {
            bytes.push(*b);
        }
        let mut i = 0;
        let block = NameBlock::read(&bytes, &mut i).unwrap();
        assert_eq!(block.name, bytes[5..]);
        assert_eq!(i, 15);
    }

    #[test]
    fn name_block_read_bad_length() {
        let mut bytes = b"NAME".to_vec();
        bytes.push(11);
        for b in b"0123456789" {
            bytes.push(*b);
        }
        let mut i = 0;
        let block = NameBlock::read(&bytes, &mut i);
        assert!(block.is_err());
    }

    #[test]
    fn info_block_read() {
        let mut bytes = b"INFO".to_vec();
        bytes.push(18);
        for b in b"0123456789abcdef" {
            bytes.push(*b);
        }
        bytes.push(0x34);
        bytes.push(0x12);
        let mut i = 0;
        let block = InfoBlock::read(&bytes, &mut i).unwrap();
        assert_eq!(block.title, bytes[5..5+16]);
        assert_eq!(block.checksum, 0x1234);
    }

    #[test]
    fn info_block_read_bad_length() {
        let mut bytes = b"INFO".to_vec();
        bytes.push(19);
        for b in b"0123456789abcdef" {
            bytes.push(*b);
        }
        bytes.push(0x34);
        bytes.push(0x12);
        let mut i = 0;
        let block = InfoBlock::read(&bytes, &mut i);
        assert!(block.is_err());
    }

    #[test]
    fn core_block_read() {
        let mut bytes = b"CORE".to_vec();
        bytes.push(0xd0);
        
        bytes.push(0x01); bytes.push(0x00); // major
        bytes.push(0x01); bytes.push(0x00); // minor
        for b in b"GD  " { // model
            bytes.push(*b);
        }
        let mut vec1 = vec![
            0x34, 0x12, // PC
            0x78, 0x56, // AF
            0xbc, 0x9a, // BC
            0xf0, 0xde, // DE
            0x12, 0x21, // HL
            0x34, 0x43, // SP
            0x01,       // IME
            0xaa,       // IE
            0x02,       // execution state
            0x00,       // reserved
        ];
        bytes.append(&mut vec1);
        // memory-mapped registers
        for _ in 0..128 {
            bytes.push(0);
        }
        let mut vec2 = vec![
            0x00, 0x80, 0x00, 0x00, // ram_size
            0xff, 0xff, 0x00, 0x00, // ram_start
            0x00, 0x20, 0x00, 0x00, // vram_size
            0xff, 0xff, 0x00, 0x00, // vram_start
            0x00, 0x20, 0x00, 0x00, // mbc_ram_size
            0xff, 0xff, 0x00, 0x00, // mbc_ram_start
            0xa0, 0x00, 0x00, 0x00, // oam_size
            0xff, 0xff, 0x00, 0x00, // oam_start
            0x80, 0x00, 0x00, 0x00, // hram_size
            0xff, 0xff, 0x00, 0x00, // hram_start
            0x40, 0x00, 0x00, 0x00, // bgp_size
            0xff, 0xff, 0x00, 0x00, // bgp_start
            0x40, 0x00, 0x00, 0x00, // obp_size
            0xff, 0xff, 0x00, 0x00, // obp_start
        ];
        bytes.append(&mut vec2);
        let ram_start_ix = bytes.len() - 52;
        let vram_start_ix = bytes.len() - 44;
        let mbc_ram_start_ix = bytes.len() - 36;
        let oam_start_ix = bytes.len() - 28;
        let hram_start_ix = bytes.len() - 20;
        let bgp_start_ix = bytes.len() - 12;
        let obp_start_ix = bytes.len() - 4;
        bytes[ram_start_ix+1] = (bytes.len() >> 8) as u8;
        bytes[ram_start_ix] = bytes.len() as u8;
        for _ in 0..0x8000 {
            bytes.push(0);
        }
        bytes[vram_start_ix+1] = (bytes.len() >> 8) as u8;
        bytes[vram_start_ix] = bytes.len() as u8;
        for _ in 0..0x2000 {
            bytes.push(0);
        }
        bytes[mbc_ram_start_ix+1] = (bytes.len() >> 8) as u8;
        bytes[mbc_ram_start_ix] = bytes.len() as u8;
        for _ in 0..0x2000 {
            bytes.push(0);
        }
        bytes[oam_start_ix+1] = (bytes.len() >> 8) as u8;
        bytes[oam_start_ix] = bytes.len() as u8;
        for _ in 0..0xa0 {
            bytes.push(0);
        }
        bytes[hram_start_ix+1] = (bytes.len() >> 8) as u8;
        bytes[hram_start_ix] = bytes.len() as u8;
        for _ in 0..0x80 {
            bytes.push(0);
        }
        bytes[bgp_start_ix+1] = (bytes.len() >> 8) as u8;
        bytes[bgp_start_ix] = bytes.len() as u8;
        for _ in 0..0x40 {
            bytes.push(0);
        }
        bytes[obp_start_ix+1] = (bytes.len() >> 8) as u8;
        bytes[obp_start_ix] = bytes.len() as u8;
        for _ in 0..0x40 {
            bytes.push(0);
        }
        let mut i = 0;
        let block = CoreBlock::read(&bytes, &mut i).unwrap();
        assert_eq!(block.major, 1);
        assert_eq!(block.minor, 1);
        assert_eq!(block.model, <[u8; 4] as std::convert::TryInto<[u8; 4]>>::try_into(*b"GD  ").unwrap());
        assert_eq!(block.pc, 0x1234);
        assert_eq!(block.af, 0x5678);
        assert_eq!(block.bc, 0x9abc);
        assert_eq!(block.de, 0xdef0);
        assert_eq!(block.hl, 0x2112);
        assert_eq!(block.sp, 0x4334);
        assert_eq!(block.ime, 0x01);
        assert_eq!(block.ie, 0xaa);
        assert_eq!(block.execution_state, 0x02);
        assert_eq!(block.reserved, 0x00);
        assert_eq!(block.memory_mapped_registers, &[0; 128]);
        assert_eq!(block.ram.len(), 0x8000);
        assert_eq!(block.vram.len(), 0x2000);
        assert_eq!(block.mbc_ram.len(), 0x2000);
        assert_eq!(block.oam.len(), 0xa0);
        assert_eq!(block.hram.len(), 0x80);
        assert_eq!(block.bgp.len(), 0x40);
        assert_eq!(block.obp.len(), 0x40);
    }

    #[test]
    fn xaom_block_read() {
        let mut bytes = b"XAOM".to_vec();
        bytes.push(0x60);
        let mut xaom_bytes = Vec::with_capacity(0x60);
        for i in 0..0x60 {
            xaom_bytes.push(i);
        }
        for b in &xaom_bytes {
            bytes.push(*b);
        }
        let mut i = 0;
        let block = XaomBlock::read(&bytes, &mut i).unwrap();
        assert_eq!(block.bytes.to_vec(), xaom_bytes);
    }

    #[test]
    fn mbc_block_read() {
        let mut bytes = b"MBC ".to_vec();
        bytes.push(0x09);
        let mut vec = vec![
            0x34, 0x12, 0x0a,
            0xff, 0xff, 0x0b,
            0xcd, 0xab, 0x0c,
        ];
        bytes.append(&mut vec);
        let mut i = 0;
        let block = MbcBlock::read(&bytes, &mut i).unwrap();
        assert_eq!(block.registers[0], (0x1234, 0x0a));
        assert_eq!(block.registers[1], (0xffff, 0x0b));
        assert_eq!(block.registers[2], (0xabcd, 0x0c));
    }

    #[test]
    fn rtc_block_read() {
        let mut bytes = b"RTC ".to_vec();
        bytes.push(0x30);
        let mut vec = vec![
            0x01, 0x00, 0x00, 0x00, // seconds
            0x02, 0x00, 0x00, 0x00, // minutes
            0x03, 0x00, 0x00, 0x00, // hours
            0x04, 0x00, 0x00, 0x00, // days_low
            0x05, 0x00, 0x00, 0x00, // days_high
            0x06, 0x00, 0x00, 0x00, // latched_seconds
            0x07, 0x00, 0x00, 0x00, // latched_minutes
            0x08, 0x00, 0x00, 0x00, // latched_hours
            0x09, 0x00, 0x00, 0x00, // latched_days_low
            0x0a, 0x00, 0x00, 0x00, // latched_days_high
        ];
        bytes.append(&mut vec);
        // unix_timestamp
        bytes.push(0xf0);
        bytes.push(0xde); 
        bytes.push(0xbc); 
        bytes.push(0x9a); 
        bytes.push(0x78);
        bytes.push(0x56); 
        bytes.push(0x34); 
        bytes.push(0x12); 
        let mut i = 0;
        let block = RtcBlock::read(&bytes, &mut i).unwrap();
        assert_eq!(block.seconds, 0x01);
        assert_eq!(block.minutes, 0x02);
        assert_eq!(block.hours, 0x03);
        assert_eq!(block.days_low, 0x04);
        assert_eq!(block.days_high, 0x05);
        assert_eq!(block.latched_seconds, 0x06);
        assert_eq!(block.latched_minutes, 0x07);
        assert_eq!(block.latched_hours, 0x08);
        assert_eq!(block.latched_days_low, 0x09);
        assert_eq!(block.latched_days_high, 0x0a);
        assert_eq!(block.unix_timestamp, 0x123456789abcdef0);
    }

    #[test]
    fn huc3_block_read() {
        let mut bytes = b"HUC3".to_vec();
        bytes.push(0x11);
        // unix_timestamp
        bytes.push(0xf0);
        bytes.push(0xde); 
        bytes.push(0xbc); 
        bytes.push(0x9a); 
        bytes.push(0x78);
        bytes.push(0x56); 
        bytes.push(0x34); 
        bytes.push(0x12);
        let mut vec = vec![
            0x34, 0x12, // rtc_minutes
            0x78, 0x56, // rtc_days
            0xbc, 0x9a, // alarm_minutes
            0xf0, 0xde, // alarm_days
        ];
        bytes.append(&mut vec);
        bytes.push(0x01); // alarm_enabled
        let mut i = 0;
        let block = Huc3Block::read(&bytes, &mut i).unwrap();
        assert_eq!(block.unix_timestamp, 0x123456789abcdef0);
        assert_eq!(block.rtc_minutes, 0x1234);
        assert_eq!(block.rtc_days, 0x5678);
        assert_eq!(block.alarm_minutes, 0x9abc);
        assert_eq!(block.alarm_days, 0xdef0);
        assert_eq!(block.alarm_enabled, 0x01);
    }

    #[test]
    fn tpp1_block_read() {
        let mut bytes = b"TPP1".to_vec();
        bytes.push(0x11);
        // unix_timestamp
        bytes.push(0xf0);
        bytes.push(0xde); 
        bytes.push(0xbc); 
        bytes.push(0x9a); 
        bytes.push(0x78);
        bytes.push(0x56); 
        bytes.push(0x34); 
        bytes.push(0x12); 
        let mut vec = vec![
            0x78, 0x56, 0x34, 0x12, // rtc
            0xf0, 0xde, 0xbc, 0x9a, // latched_rtc 
        ];
        bytes.append(&mut vec);
        bytes.push(0xcc); // mr4
        let mut i = 0;
        let block = Tpp1Block::read(&bytes, &mut i).unwrap();
        assert_eq!(block.unix_timestamp, 0x123456789abcdef0);
        assert_eq!(block.rtc, 0x12345678);
        assert_eq!(block.latched_rtc, 0x9abcdef0);
        assert_eq!(block.mr4, 0xcc);
    }

    #[test]
    fn mbc7_block_read() {
        let mut bytes = b"MBC7".to_vec();
        bytes.push(0x0a);
        bytes.push(0xcc); // flags
        bytes.push(0xee); // argument_bits_left
        let mut vec = vec![
            0x34, 0x12, // eeprom_command
            0x78, 0x56, // pending_bits
            0xbc, 0x9a, // latched_gyro_x
            0xf0, 0xde, // latched_gyro_y
        ];
        bytes.append(&mut vec);
        let mut i = 0;
        let block = Mbc7Block::read(&bytes, &mut i).unwrap();
        assert_eq!(block.flags, 0xcc);
        assert_eq!(block.argument_bits_left, 0xee);
        assert_eq!(block.eeprom_command, 0x1234);
        assert_eq!(block.pending_bits, 0x5678);
        assert_eq!(block.latched_gyro_x, 0x9abc);
        assert_eq!(block.latched_gyro_y, 0xdef0);
    }
}
