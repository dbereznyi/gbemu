use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use crate::gameboy::{*};

struct Tile {
    pub pixels: [u8; 8*8],
}

impl Tile {
    pub fn new(bytes: &[u8]) -> Tile {
        let mut tile = Tile {
            pixels: [0; 8*8],
        };

        let palette: [u8; 4] = [255, 127, 63, 0];

        // Each line of 8 pixels is represented by 2 bytes:
        // 1011_1100 <== low byte
        // 0001_0011 <== high byte
        // Each corresponding pair of bits from the high byte and the low byte forms an index into
        // the palette

        for i in (0..16).step_by(2) {
            let selector = 1;
            
            for bit_pos in (0..8).rev() {
                let high_bit = bytes[i+1] & (selector << bit_pos);
                let low_bit = bytes[i] & (selector << bit_pos);
                let palette_ix = 2*(high_bit >> bit_pos) + (low_bit >> bit_pos);
                
                let pixel_ix = (i/2)*8 + (7-bit_pos);
                let color = palette[palette_ix as usize];

                tile.pixels[pixel_ix] = color;
            }
        }

        tile
    }
}

pub fn run_ppu(
    vram_m: Arc<Mutex<[u8; 0x2000]>>, 
    oam_m: Arc<Mutex<[u8; 0xa0]>>, 
    io_ports_m: Arc<Mutex<[u8; 0x4c]>>,
    screen_m: Arc<Mutex<[[u8; 160]; 144]>>) {
    let mut background: [[u8; 256]; 256] = [[255; 256]; 256];

    loop {
        for y in 0..144 {
            // Transfer data from OAM
            let mut io_ports = io_ports_m.lock().unwrap();
            io_ports[IO_STAT] &= !STAT_MODE;
            io_ports[IO_STAT] |= STAT_MODE_OAM;
            drop(io_ports);
            // TODO access OAM when we handle sprites
            thread::sleep(Duration::new(0, 19000)); // roughly the time of OAM access (19 microsecs)

            // Transfer data from VRAM
            let mut io_ports = io_ports_m.lock().unwrap();
            io_ports[IO_STAT] &= !STAT_MODE;
            io_ports[IO_STAT] |= STAT_MODE_TRANSFER;
            let scx = io_ports[IO_SCX] as usize;
            let scy = io_ports[IO_SCY] as usize;
            let lcdc = io_ports[IO_LCDC];
            drop(io_ports);

            let vram = &vram_m.lock().unwrap();
            let bg_tile_data = 
                if lcdc & LCDC_TILE_DATA > 0 { 
                    &vram[0x0000..0x1000] 
                } else {
                    &vram[0x0800..0x1800]
                };
            let bg_tile_map = 
                if lcdc & LCDC_BG_TILE_MAP > 0 {
                    &vram[0x1c00..0x2000]
                } else {
                    &vram[0x1800..0x1c00]
                };
            let mut tiles = Vec::with_capacity(32*32);
            for i in (0x0000..0x1000).step_by(16) {
                tiles.push(Tile::new(&bg_tile_data[i..i+16]));
            }
            for i in 0..32*32 {
                let tile = &tiles[bg_tile_map[i] as usize];
                for y in 0..8 {
                    for x in 0..8 {
                        let bg_y = (i/32)*8 + y;
                        let bg_x = (i%32)*8 + x;
                        background[bg_y][bg_x] = tile.pixels[y*8 + x];
                    }
                }
            }
            drop(vram);

            // Draw pixels to the LCD screen
            let mut screen = screen_m.lock().unwrap();
            for x in 0..160 {
                screen[y][x] = background[(scy + y) % 256][(scx + x) % 256];
            }
            drop(screen);

            // Enter HBlank period
            let mut io_ports = io_ports_m.lock().unwrap();
            io_ports[IO_STAT] &= !STAT_MODE;
            io_ports[IO_STAT] |= STAT_MODE_HBLANK;
            drop(io_ports);

            // TODO trigger an HBlank interrupt if needed

            thread::sleep(Duration::new(0, 48600)); // roughly the time of HBlank interval (48.6 microsecs)
        }
        
        let mut io_ports = io_ports_m.lock().unwrap();
        io_ports[IO_STAT] &= !STAT_MODE;
        io_ports[IO_STAT] |= STAT_MODE_VBLANK;
        // TODO trigger a VBlank interrupt if needed
        drop(io_ports);

        thread::sleep(Duration::new(0, 1087188)); // roughly the time of VBlank interval (4560 clock cycles)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tile() {
        let bytes: [u8; 16] = [
            0x7c, 0x7c, 
            0x00, 0xc6, 
            0xc6, 0x00, 
            0x00, 0xfe, 
            0xc6, 0xc6, 
            0x00, 0xc6, 
            0xc6, 0x00, 
            0x00, 0x00
        ];

        let tile = Tile::new(&bytes);

        let palette: [u8; 4] = [255, 127, 63, 0];

        let mut pixels_expected: [u8; 8*8] = [
            0, 3, 3, 3, 3, 3, 0, 0,
            2, 2, 0, 0, 0, 2, 2, 0,
            1, 1, 0, 0, 0, 1, 1, 0,
            2, 2, 2, 2, 2, 2, 2, 0,
            3, 3, 0, 0, 0, 3, 3, 0,
            2, 2, 0, 0, 0, 2, 2, 0,
            1, 1, 0, 0, 0, 1, 1, 0,
            0, 0, 0, 0, 0, 0, 0, 0,
        ];

        for y in 0..8 {
            for x in 0..8 {
                let i = y*8 + x;
                pixels_expected[i] = palette[pixels_expected[i] as usize];
            }
        }

        assert_eq!(tile.pixels, pixels_expected);
    }
}
