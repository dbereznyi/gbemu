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
        let io_ports = ppu.io_ports.lock().unwrap();
        let wy = io_ports[IO_WY] as usize; // WY is only updated once per frame
        drop(io_ports);
        let mut curr_window_line = 0;

        for y in 0..144 {
            // Transfer data from OAM
            let mut io_ports = ppu.io_ports.lock().unwrap();
            io_ports[IO_LY] = y as u8;
            if io_ports[IO_LY] == io_ports[IO_LYC] {
                io_ports[IO_STAT] |= STAT_LYC_SET;
            } else {
                io_ports[IO_STAT] &= !STAT_LYC_SET;
            }
            io_ports[IO_STAT] &= !STAT_MODE;
            io_ports[IO_STAT] |= STAT_MODE_OAM;
            if ppu.ime.load(Ordering::Relaxed) && io_ports[IO_IE] & LCDC > 0 && io_ports[IO_STAT] & STAT_INT_M10 > 0 {
                io_ports[IO_IF] |= LCDC;
                let mut interrupted = mutex.lock().unwrap();
                *interrupted = true;
                cvar.notify_one();
            }
            drop(io_ports);
            // TODO access OAM when we handle sprites
            thread::sleep(Duration::new(0, 19000)); // roughly the time of OAM access (19 microsecs)

            // Transfer data from VRAM
            let mut io_ports = ppu.io_ports.lock().unwrap();
            io_ports[IO_STAT] &= !STAT_MODE;
            io_ports[IO_STAT] |= STAT_MODE_TRANSFER;
            let scx = io_ports[IO_SCX];
            let scy = io_ports[IO_SCY];
            let wx = io_ports[IO_WX] as usize;
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
            let win_tile_map =
                if lcdc & LCDC_WIN_TILE_MAP > 0 {
                    &vram[0x1c00..0x2000]
                } else {
                    &vram[0x1800..0x1c00]
                };

            // Draw pixels to the screen
            let mut screen = ppu.screen.lock().unwrap();
            for x in 0..160 {
                const PALETTE: [u8; 4] = [255, 127, 63, 0];
                screen[y][x] = PALETTE[0];

                if lcdc & LCDC_BG_DISP > 0 {  
                    // figure out which tile we are drawing 
                    let scrolled_x = (Wrapping(x as u8) + Wrapping(scx)).0;
                    let scrolled_y = (Wrapping(y as u8) + Wrapping(scy)).0;
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
                    let bgp_mask = 0b11 << (palette_ix * 2);
                    let bgp_palette_ix = (bgp & bgp_mask) >> (palette_ix * 2);
                    screen[y][x] = PALETTE[bgp_palette_ix as usize];
                }

                if lcdc & LCDC_WIN_DISP > 0 {
                    if x + 7 >= wx && x + 7 <= 166 && y >= wy && y <= 143 {
                        // figure out which tile we are drawing
                        let window_x = x - (wx - 7);
                        let current_tile_ix = (curr_window_line / 8)*32 + (window_x as usize / 8);
                        // grab data for current tile row    
                        let tile_data_ix = win_tile_map[current_tile_ix] as usize;
                        let row_ix = curr_window_line % 8;
                        let col_ix = window_x % 8;
                        let row_start = (tile_data_ix * 16) + (row_ix * 2);
                        let row = &bg_tile_data[row_start..row_start+2];
                        // determine palette index from high and low bytes
                        let col_mask = 1 << (7 - col_ix);
                        let high_bit = (row[1] & col_mask) >> (7 - col_ix);
                        let low_bit = (row[0] & col_mask) >> (7 - col_ix);
                        let palette_ix = 2*high_bit + low_bit;
                        // finally, determine pixel color using BGP register lookup
                        let bgp_mask = 0b11 << (palette_ix * 2);
                        let bgp_palette_ix = (bgp & bgp_mask) >> (palette_ix * 2);
                        screen[y][x] = PALETTE[bgp_palette_ix as usize];

                        // If the window gets disabled during HBlank and then re-enabled later on,
                        // we want to continue drawing from where we left off
                        if x == 159 {
                            curr_window_line += 1;
                        }
                    } 
                }

                // TODO draw sprites
            }
            drop(vram);
            drop(screen);

            // Enter HBlank period, and trigger an interrupt if Mode 00 interrupts enabled in STAT
            // or LYC incident interrupts enabled in STAT and LY=LYC

            let mut io_ports = ppu.io_ports.lock().unwrap();
            io_ports[IO_STAT] &= !STAT_MODE;
            io_ports[IO_STAT] |= STAT_MODE_HBLANK;
            let int_on_hblank = io_ports[IO_STAT] & STAT_INT_M00 > 0;
            let int_on_lyc_incident = io_ports[IO_STAT] & STAT_INT_LYC > 0;
            let lyc_incident = io_ports[IO_STAT] & STAT_LYC_SET > 0;
            if ppu.ime.load(Ordering::Relaxed)
                && io_ports[IO_IE] & LCDC > 0 
                && (int_on_hblank || (int_on_lyc_incident && lyc_incident)) {
                io_ports[IO_IF] |= LCDC;
                let mut interrupted = mutex.lock().unwrap();
                *interrupted = true;
                cvar.notify_one();
            }
            drop(io_ports);
            thread::sleep(Duration::new(0, 48600)); // roughly the time of HBlank interval (48.6 microsecs)
        }
        
        // Enter VBlank period, and trigger an interrupt if VBlank interrupts enabled in IE
        // or if LCDC interrupts enabled in IE and Mode 01 interrupts enabled in STAT

        let mut io_ports = ppu.io_ports.lock().unwrap();
        io_ports[IO_STAT] &= !STAT_MODE;
        io_ports[IO_STAT] |= STAT_MODE_VBLANK;
        io_ports[IO_LY] = 144;
        let int_on_vblank = io_ports[IO_IE] & VBLANK > 0;
        let int_on_m01 = (io_ports[IO_IE] & LCDC > 0) && (io_ports[IO_STAT] & STAT_INT_M01 > 0);
        if ppu.ime.load(Ordering::Relaxed) && (int_on_vblank || int_on_m01) {
            io_ports[IO_IF] |= VBLANK;
            let mut interrupted = mutex.lock().unwrap();
            *interrupted = true;
            cvar.notify_one();
        }
        drop(io_ports);
        thread::sleep(Duration::new(0, 1087188)); // roughly the time of VBlank interval (4560 clock cycles)
    }
}
