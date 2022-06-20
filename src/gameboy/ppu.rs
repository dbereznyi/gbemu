use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicU8, AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use std::num::Wrapping;

use crate::gameboy::{*};

pub struct Ppu {
    pub vram: Arc<Mutex<[u8; 0x2000]>>, 
    pub oam: Arc<Mutex<[u8; 0xa0]>>, 
    pub io_ports: Arc<Mutex<[u8; 0x4d]>>,
    pub screen: Arc<Mutex<[[u8; 160]; 144]>>,
    pub ime: Arc<AtomicBool>,
    pub interrupt_received: Arc<(Mutex<bool>, Condvar)>,
}

pub fn run_ppu(ppu: &mut Ppu) {
    let (mutex, cvar) = &*ppu.interrupt_received;

    loop {
        for y in 0..144 {
            // Transfer data from OAM
            let mut io_ports = ppu.io_ports.lock().unwrap();
            io_ports[IO_STAT] &= !STAT_MODE;
            io_ports[IO_STAT] |= STAT_MODE_OAM;
            drop(io_ports);
            // TODO access OAM when we handle sprites
            thread::sleep(Duration::new(0, 19000)); // roughly the time of OAM access (19 microsecs)

            // Transfer data from VRAM
            let mut io_ports = ppu.io_ports.lock().unwrap();
            io_ports[IO_STAT] &= !STAT_MODE;
            io_ports[IO_STAT] |= STAT_MODE_TRANSFER;
            let scx = io_ports[IO_SCX] as usize;
            let scy = io_ports[IO_SCY] as usize;
            let lcdc = io_ports[IO_LCDC];
            let bgp = io_ports[IO_BGP];
            drop(io_ports);

            let vram = &ppu.vram.lock().unwrap();
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

            // Draw pixels to the screen
            let mut screen = ppu.screen.lock().unwrap();
            for x in 0..160 {
                if (lcdc & LCDC_BG_DISP) > 0 {  
                    // figure out which tile we are drawing 
                    let scrolled_x = (Wrapping(x as u8) + Wrapping(scx as u8)).0;
                    let scrolled_y = (Wrapping(y as u8) + Wrapping(scy as u8)).0;
                    let current_tile_ix = (scrolled_y as usize / 8)*32 + (scrolled_x as usize / 8);
                    // grab data for current tile row
                    let tile_data_ix = bg_tile_map[current_tile_ix] as usize;
                    let row_ix = (scrolled_y % 8) as usize;
                    let col_ix = (scrolled_x % 8) as usize;
                    let row_start = (tile_data_ix * 16) + (row_ix * 2);
                    let row = &bg_tile_data[row_start..row_start+2];
                    // determine palette index from high and low bytes
                    let col_mask = 1 << (7 - col_ix);
                    let high_bit = (row[1] & col_mask) >> (7 - col_ix);
                    let low_bit = (row[0] & col_mask) >> (7 - col_ix);
                    let palette_ix = 2*high_bit + low_bit;
                    // finally, determine pixel color using BGP register lookup
                    const PALETTE: [u8; 4] = [255, 127, 63, 0];
                    let bgp_mask = 0b11 << (palette_ix * 2);
                    let bgp_palette_ix = (bgp & bgp_mask) >> (palette_ix * 2);
                    screen[y][x] = PALETTE[bgp_palette_ix as usize];
                } else {
                    screen[y][x] = 255;
                }

                // TODO do similar stuff for sprite and window layers
            }
            drop(vram);
            drop(screen);

            // Enter HBlank period
            let mut io_ports = ppu.io_ports.lock().unwrap();
            io_ports[IO_STAT] &= !STAT_MODE;
            io_ports[IO_STAT] |= STAT_MODE_HBLANK;
            drop(io_ports);

            // TODO trigger an HBlank interrupt if needed

            thread::sleep(Duration::new(0, 48600)); // roughly the time of HBlank interval (48.6 microsecs)
        }
        
        let mut io_ports = ppu.io_ports.lock().unwrap();
        io_ports[IO_STAT] &= !STAT_MODE;
        io_ports[IO_STAT] |= STAT_MODE_VBLANK;
        if ppu.ime.load(Ordering::Relaxed) && (io_ports[IO_IE] & VBLANK) > 0 {
            io_ports[IO_IF] |= VBLANK;
            let mut interrupted = mutex.lock().unwrap();
            *interrupted = true;
            cvar.notify_one();
        }
        drop(io_ports);

        thread::sleep(Duration::new(0, 1087188)); // roughly the time of VBlank interval (4560 clock cycles)
    }
}
