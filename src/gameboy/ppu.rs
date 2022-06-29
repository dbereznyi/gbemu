use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration};
use std::num::{Wrapping};
use crate::gameboy::gameboy::{*};

pub const PALETTE_GREY: [(u8,u8,u8); 4] = [(255,255,255), (127,127,127), (63,63,63), (0,0,0)];
pub const PALETTE_RED: [(u8,u8,u8); 4] = [(255,0,0), (127,0,0), (63,0,0), (0,0,0)];
pub const PALETTE_GREEN: [(u8,u8,u8); 4] = [(0,255,0), (0,127,0), (0,63,0), (0,0,0)];
pub const PALETTE_BLUE: [(u8,u8,u8); 4] = [(0,0,255), (0,0,127), (0,0,63), (0,0,0)];

// Sprite attribute flags
const OBJ_PRIORITY: u8 = 0b1000_0000;
const OBJ_Y_FLIP: u8   = 0b0100_0000;
const OBJ_X_FLIP: u8   = 0b0010_0000;
const OBJ_PALETTE: u8  = 0b0001_0000;

/// Object (sprite) attributes
struct ObjAttr {
    pub y: u8,
    pub x: u8,
    pub tile_number: u8,
    pub flags: u8,
}

impl ObjAttr {
    pub fn new(bytes: &[u8]) -> ObjAttr {
        ObjAttr {
            y: bytes[0],
            x: bytes[1],
            tile_number: bytes[2],
            flags: bytes[3],
        }
    }
}

pub struct Ppu {
    pub vram: Arc<Mutex<[u8; 0x2000]>>, 
    pub oam: Arc<Mutex<[u8; 0xa0]>>, 
    pub io_ports: Arc<Mutex<[u8; 0x4d]>>,
    pub screen: Arc<Mutex<[[(u8,u8,u8); 160]; 144]>>,
    pub ime: Arc<AtomicBool>,
    pub interrupt_received: Arc<(Mutex<bool>, Condvar)>,
    pub palette: [(u8,u8,u8); 4],
}

pub fn run_ppu(ppu: &mut Ppu) {
    let (mutex, cvar) = &*ppu.interrupt_received;

    loop {
        let io_ports = ppu.io_ports.lock().unwrap();
        let lcd_is_off = io_ports[IO_LCDC] & LCDC_ON == 0; // LCD should only be turned off in VBlank
        let wy = io_ports[IO_WY] as usize; // WY is only checked once per frame
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
            let oam = ppu.oam.lock().unwrap();
            let mut obj_attrs: Vec<(usize, ObjAttr)> = Vec::with_capacity(40);
            for i in (0..0xa0).step_by(4) {
                obj_attrs.push((i, ObjAttr::new(&oam[i..i+4])));
            }
            obj_attrs.sort_by(|(a_i, a_attr), (b_i, b_attr)| {
                if a_attr.x != b_attr.x { 
                    // Sort by X coord, descending (because sprites at end of the Vec will get
                    // drawn on top)
                    b_attr.x.cmp(&a_attr.x)
                } else {
                    // In the case of equal X coords, lower-entry sprites are drawn on top
                    b_i.cmp(a_i)
                }
            });
            thread::sleep(Duration::new(0, 19000)); // roughly the time of OAM access (19 microsecs)
            drop(oam);

            // Transfer data from VRAM
            let mut io_ports = ppu.io_ports.lock().unwrap();
            io_ports[IO_STAT] &= !STAT_MODE;
            io_ports[IO_STAT] |= STAT_MODE_TRANSFER;
            let scx = io_ports[IO_SCX];
            let scy = io_ports[IO_SCY];
            let wx = io_ports[IO_WX] as usize;
            let lcdc = io_ports[IO_LCDC];
            let bgp = io_ports[IO_BGP];
            let obp0 = io_ports[IO_OBP0];
            let obp1 = io_ports[IO_OBP1];
            drop(io_ports);

            let vram = &ppu.vram.lock().unwrap();
            let bg_tile_data = 
                if lcdc & LCDC_TILE_DATA > 0 { 
                    &vram[0x0000..0x1000] 
                } else {
                    &vram[0x0800..0x1800]
                };
            let obj_tile_data = &vram[0x0000..0x1000]; // OBJ tile data is always at 0x8000-0x8fff
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
                screen[y][x] = ppu.palette[0];

                if lcd_is_off {
                    continue;
                }

                if lcdc & LCDC_BG_DISP > 0 {  
                    // figure out which tile we are drawing 
                    let scrolled_x = (Wrapping(x as u8) + Wrapping(scx)).0;
                    let scrolled_y = (Wrapping(y as u8) + Wrapping(scy)).0;
                    let current_tile_ix = (scrolled_y as usize / 8)*32 + (scrolled_x as usize / 8);
                    // grab data for current tile row
                    let tile_data_ix = 
                        if LCDC & LCDC_TILE_DATA > 0 {
                            bg_tile_map[current_tile_ix] as usize
                        } else {
                            // If tile data is at 0x9000, these tile numbers go from -127 to 128
                            // So a value of 0x80 refers to tile #0, and 0x00 refers to tile #128
                            (Wrapping(bg_tile_map[current_tile_ix]) + Wrapping(128)).0 as usize
                        };
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
                    screen[y][x] = ppu.palette[bgp_palette_ix as usize];
                }

                if lcdc & LCDC_WIN_DISP > 0 {
                    if x + 7 >= wx && x + 7 <= 166 && y >= wy && y <= 143 {
                        // figure out which tile we are drawing
                        let window_x = x - (wx - 7);
                        let current_tile_ix = (curr_window_line / 8)*32 + (window_x as usize / 8);
                        // grab data for current tile row    
                        let tile_data_ix = 
                            if LCDC & LCDC_TILE_DATA > 0 {
                                win_tile_map[current_tile_ix] as usize
                            } else {
                                // If tile data is at 0x9000, these tile numbers go from -127 to 128
                                // So a value of 0x80 refers to tile #0, and 0x00 refers to tile #128
                                (Wrapping(win_tile_map[current_tile_ix]) + Wrapping(128)).0 as usize
                            };
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
                        screen[y][x] = ppu.palette[bgp_palette_ix as usize];

                        // If the window gets disabled during HBlank and then re-enabled later on,
                        // we want to continue drawing from where we left off
                        if x == 159 {
                            curr_window_line += 1;
                        }
                    } 
                }

                if lcdc & LCDC_OBJ_DISP > 0 {
                    for (_, obj) in obj_attrs.iter() {
                        let obj_x = obj.x as usize;
                        let obj_y = obj.y as usize;

                        let x_in_range = x >= obj_x - 8 && x < obj_x;
                        let y_in_range =
                            if lcdc & LCDC_OBJ_SIZE > 0 {
                                y >= obj_y - 16 && y < obj_y
                            } else {
                                y >= obj_y - 16 && y < obj_y - 8
                            };

                        if !x_in_range || !y_in_range {
                            continue;
                        }

                        let row_ix = y - (obj_y - 16);
                        let col_ix = x - (obj_x - 8);
                        // apply Y flip and X flip if set
                        let row_ix = 
                            if obj.flags & OBJ_Y_FLIP > 0 {
                                if lcdc & LCDC_OBJ_SIZE > 0 { 
                                    15 - row_ix
                                } else {
                                    7 - row_ix
                                }
                            } else {
                                row_ix
                            };
                        let col_ix =
                            if obj.flags & OBJ_X_FLIP > 0 {
                                7 - col_ix
                            } else {
                                col_ix
                            };
                        let tile_number = 
                            if lcdc & LCDC_OBJ_SIZE > 0 {
                                (obj.tile_number & 0b1111_1110) as usize
                            } else {
                                obj.tile_number as usize
                            };
                        let row_start = (tile_number * 16) + (row_ix * 2);
                        let row = &obj_tile_data[row_start..row_start+2];
                        // determine palette index from high and low bytes
                        let col_mask = 1 << (7 - col_ix);
                        let high_bit = (row[1] & col_mask) >> (7 - col_ix);
                        let low_bit = (row[0] & col_mask) >> (7 - col_ix);
                        let palette_ix = 2*high_bit + low_bit;
                        // if this pixel is transparent, skip drawing
                        if palette_ix == 0 {
                            continue;
                        }
                        // determine pixel color using appropriate OBP register lookup
                        let obp_mask = 0b11 << (palette_ix * 2);
                        let obp_reg = if obj.flags & OBJ_PALETTE > 0 { obp1 } else { obp0 };
                        let obp_palette_ix = (obp_reg & obp_mask) >> (palette_ix * 2);
                        // if priority bit set and underlying pixel is not color 0,
                        // then don't draw this pixel 
                        let priority = obj.flags & OBJ_PRIORITY > 0;
                        if priority && screen[y][x] != ppu.palette[0] {
                            continue;
                        }
                        screen[y][x] = ppu.palette[obp_palette_ix as usize];
                    }
                }
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
        // TODO properly simulate LY increasing from 144 to 153 throughout VBlank
        thread::sleep(Duration::new(0, 1_087_188)); // roughly the time of VBlank interval (1.09ms)
    }
}
