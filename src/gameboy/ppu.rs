use std::sync::{Arc, Mutex, Condvar};
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};
use std::num::{Wrapping};
use std::convert::TryInto;
use crate::gameboy::gameboy::{*};
use crate::gameboy::debug_info::{DebugInfoPpu};
use crate::gameboy::utils::{sleep_precise};

pub const PALETTE_GREY: [(u8,u8,u8); 4] = [(255,255,255), (127,127,127), (63,63,63), (0,0,0)];
pub const PALETTE_RED: [(u8,u8,u8); 4] = [(255,0,0), (127,0,0), (63,0,0), (0,0,0)];
pub const PALETTE_GREEN: [(u8,u8,u8); 4] = [(0,255,0), (0,127,0), (0,63,0), (0,0,0)];
pub const PALETTE_BLUE: [(u8,u8,u8); 4] = [(0,0,255), (0,0,127), (0,0,63), (0,0,0)];

const OAM_TIME: Duration = Duration::from_nanos(19_000);
const DRAW_TIME: Duration = Duration::from_nanos(41_000);
const HBLANK_TIME: Duration = Duration::from_nanos(48_600);
const LINE_TIME: Duration = Duration::from_nanos(108_718);
const FRAME_TIME: Duration  = Duration::from_nanos(16_750_000);

const OBJ_PRIORITY: u8 = 0b1000_0000;
const OBJ_Y_FLIP: u8   = 0b0100_0000;
const OBJ_X_FLIP: u8   = 0b0010_0000;
const OBJ_PALETTE: u8  = 0b0001_0000;

#[derive(Debug, Copy, Clone)]
struct ObjAttr {
    pub y: u8,
    pub x: u8,
    pub tile_number: u8,
    pub flags: u8,
}

impl ObjAttr {
    pub fn new(bytes: &[u8]) -> Self {
        Self {
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
    pub io_ports: Arc<IoPorts>,
    pub screen: Arc<Mutex<[[(u8,u8,u8); 160]; 144]>>,
    pub ime: Arc<AtomicBool>,
    pub interrupt_received: Arc<(Mutex<bool>, Condvar)>,
    pub palette: [(u8,u8,u8); 4],
    pub step_mode: Arc<AtomicBool>,
}

impl Ppu {
    pub fn new(gb: &Gameboy, palette: [(u8, u8, u8); 4]) -> Self {
        Self {
            vram: gb.vram.clone(),
            oam: gb.oam.clone(),
            io_ports: gb.io_ports.clone(),
            screen: gb.screen.clone(),
            ime: gb.ime.clone(),
            interrupt_received: Arc::clone(&gb.interrupt_received),
            palette: palette,
            step_mode: gb.debug.step_mode.clone(),
        }
    }
}

pub fn run_ppu(ppu: &mut Ppu, debug_info: DebugInfoPpu) {
    let (mutex, cvar) = &*ppu.interrupt_received;
    let io_ports = &ppu.io_ports;

    let mut obj_attrs: Vec<(usize, ObjAttr)> = Vec::with_capacity(40);
    // This is to get around having to init the vec with dummy data that we just overwrite in the
    // first run of the PPU.
    unsafe { obj_attrs.set_len(40); }
    
    // Give the CPU a bit to start up
    thread::sleep(Duration::from_nanos(1000));

    loop {
        let frame_start = Instant::now();

        let lcd_is_off = io_ports.read(IO_LCDC) & LCDC_ON == 0; // LCD should only be turned off in VBlank
        let wy = io_ports.read(IO_WY) as usize; // WY is only checked once per frame

        let mut curr_window_line = 0;

        io_ports.write(IO_LY, 0);

        wait_if_debug_break(ppu);

        for y in 0..144 {
            let oam_start = Instant::now();
            io_ports.and(IO_STAT, !STAT_MODE);
            io_ports.or(IO_STAT, STAT_MODE_OAM);
            let int_on_m10 =
                io_ports.read(IO_IE) & INT_LCDC > 0 && io_ports.read(IO_STAT) & STAT_INT_M10 > 0;
            if ppu.ime.load(Ordering::Relaxed) && int_on_m10 {
                io_ports.or(IO_IF, INT_LCDC);
                let mut interrupted = mutex.lock().unwrap();
                *interrupted = true;
                cvar.notify_one();
            }
            let oam = ppu.oam.lock().unwrap();
            for (i, j) in (0..160).step_by(4).enumerate() {
                obj_attrs[i] = (j, ObjAttr::new(&oam[j..j+4]));
            }
            // Objects with smaller x coords have priority. If x coords are equal, the object that
            // comes earlier in OAM has priority.
            obj_attrs.sort_by(|(a_i, a_attr), (b_i, b_attr)| {
                if a_attr.x != b_attr.x { 
                    a_attr.x.cmp(&b_attr.x)
                } else {
                    a_i.cmp(b_i)
                }
            });
            let lcdc = io_ports.read(IO_LCDC);
            // Up to 10 objects can be drawn per scanline.
            let mut obj_attrs_line: Vec<ObjAttr> = Vec::with_capacity(10);
            {
                for (_, obj) in obj_attrs.iter() {
                    let obj_y = obj.y as usize;

                    let y_in_range =
                        if lcdc & LCDC_OBJ_SIZE > 0 {
                            y >= (Wrapping(obj_y) - Wrapping(16)).0 && y < obj_y
                        } else {
                            y >= (Wrapping(obj_y) - Wrapping(16)).0 && y < (Wrapping(obj_y) - Wrapping(8)).0
                        };

                    if y_in_range {
                        obj_attrs_line.push(*obj);
                        if obj_attrs_line.len() == 10 { break; }
                    }
                }
            }
            // Higher-priority objects should come later so that they will be drawn on top of
            // lower-priority objects.
            obj_attrs_line.reverse();
            wait_if_debug_break(ppu);
            sleep_precise(OAM_TIME.checked_sub(oam_start.elapsed()).unwrap_or(Duration::ZERO));
            debug_info.oam_time_nanos.store(oam_start.elapsed().as_nanos() as u64, Ordering::Relaxed);
            wait_if_debug_break(ppu);
            drop(oam);

            let draw_start = Instant::now();
            io_ports.and(IO_STAT, !STAT_MODE);
            io_ports.or(IO_STAT, STAT_MODE_TRANSFER);
            let scx = io_ports.read(IO_SCX);
            let scy = io_ports.read(IO_SCY);
            let wx = io_ports.read(IO_WX) as usize;
            let bgp = io_ports.read(IO_BGP);
            let obp0 = io_ports.read(IO_OBP0);
            let obp1 = io_ports.read(IO_OBP1);
            let vram = &ppu.vram.lock().unwrap();
            let bg_tile_data = 
                if lcdc & LCDC_TILE_DATA > 0 { 
                    &vram[0x0000..0x1000] 
                } else {
                    &vram[0x0800..0x1800]
                };
            let obj_tile_data = &vram[0x0000..0x1000];
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

            let mut screen = ppu.screen.lock().unwrap();
            for x in 0..160 {
                wait_if_debug_break(ppu);
                screen[y][x] = ppu.palette[0];

                if lcd_is_off {
                    continue;
                }

                if lcdc & LCDC_BG_DISP > 0 {  
                    let scrolled_x = (Wrapping(x as u8) + Wrapping(scx)).0;
                    let scrolled_y = (Wrapping(y as u8) + Wrapping(scy)).0;
                    let current_tile_ix = (scrolled_y as usize / 8)*32 + (scrolled_x as usize / 8);
                    let tile_data_ix = 
                        if lcdc & LCDC_TILE_DATA > 0 {
                            bg_tile_map[current_tile_ix] as usize
                        } else {
                            (Wrapping(bg_tile_map[current_tile_ix]) + Wrapping(128)).0 as usize
                        };
                    let row_ix = (scrolled_y % 8) as usize;
                    let col_ix = (scrolled_x % 8) as usize;
                    let row_start = (tile_data_ix * 16) + (row_ix * 2);
                    let row = &bg_tile_data[row_start..row_start+2];
                    let col_mask = 1 << (7 - col_ix);
                    let high_bit = (row[1] & col_mask) >> (7 - col_ix);
                    let low_bit = (row[0] & col_mask) >> (7 - col_ix);
                    let palette_ix = 2*high_bit + low_bit;
                    let bgp_mask = 0b11 << (palette_ix * 2);
                    let bgp_palette_ix = (bgp & bgp_mask) >> (palette_ix * 2);
                    screen[y][x] = ppu.palette[bgp_palette_ix as usize];
                }

                if lcdc & LCDC_WIN_DISP > 0 {
                    if x + 7 >= wx && x + 7 <= 166 && y >= wy && y <= 143 {
                        let window_x = x - (wx - 7);
                        let current_tile_ix = (curr_window_line / 8)*32 + (window_x as usize / 8);
                        let tile_data_ix = 
                            if lcdc & LCDC_TILE_DATA > 0 {
                                win_tile_map[current_tile_ix] as usize
                            } else {
                                (Wrapping(win_tile_map[current_tile_ix]) + Wrapping(128)).0 as usize
                            };
                        let row_ix = curr_window_line % 8;
                        let col_ix = window_x % 8;
                        let row_start = (tile_data_ix * 16) + (row_ix * 2);
                        let row = &bg_tile_data[row_start..row_start+2];
                        let col_mask = 1 << (7 - col_ix);
                        let high_bit = (row[1] & col_mask) >> (7 - col_ix);
                        let low_bit = (row[0] & col_mask) >> (7 - col_ix);
                        let palette_ix = 2*high_bit + low_bit;
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
                    for obj in obj_attrs_line.iter() {
                        let obj_x = obj.x as usize;
                        let obj_y = obj.y as usize;
                        let x_in_range = x >= obj_x - 8 && x < obj_x;
                        if !x_in_range {
                            continue;
                        }

                        let row_ix = y - (obj_y - 16);
                        let col_ix = x - (obj_x - 8);
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
                        let col_mask = 1 << (7 - col_ix);
                        let high_bit = (row[1] & col_mask) >> (7 - col_ix);
                        let low_bit = (row[0] & col_mask) >> (7 - col_ix);
                        let palette_ix = 2*high_bit + low_bit;
                        if palette_ix == 0 {
                            continue;
                        }
                        let obp_mask = 0b11 << (palette_ix * 2);
                        let obp_reg = if obj.flags & OBJ_PALETTE > 0 { obp1 } else { obp0 };
                        let obp_palette_ix = (obp_reg & obp_mask) >> (palette_ix * 2);
                        let priority = obj.flags & OBJ_PRIORITY > 0;
                        if priority && screen[y][x] != ppu.palette[0] {
                            continue;
                        }
                        screen[y][x] = ppu.palette[obp_palette_ix as usize];
                    }
                }
            }
            wait_if_debug_break(ppu);
            sleep_precise(DRAW_TIME.checked_sub(draw_start.elapsed()).unwrap_or(Duration::ZERO));
            wait_if_debug_break(ppu);
            drop(vram);
            drop(screen);

            // HBlank

            io_ports.and(IO_STAT, !STAT_MODE);
            io_ports.or(IO_STAT, STAT_MODE_HBLANK);
            let int_on_hblank = io_ports.read(IO_STAT) & STAT_INT_M00 > 0;
            let int_on_lyc_incident = io_ports.read(IO_STAT) & STAT_INT_LYC > 0;
            let lyc_incident = io_ports.read(IO_STAT) & STAT_LYC_SET > 0;
            if ppu.ime.load(Ordering::Relaxed) && io_ports.read(IO_IE) & INT_LCDC > 0 && (int_on_hblank || (int_on_lyc_incident && lyc_incident)) {
                io_ports.or(IO_IF, INT_LCDC);
                let mut interrupted = mutex.lock().unwrap();
                *interrupted = true;
                cvar.notify_one();
            }
            wait_if_debug_break(ppu);
            sleep_precise(HBLANK_TIME);
            wait_if_debug_break(ppu);

            io_ports.add(IO_LY, 1);
            if io_ports.read(IO_LY) == io_ports.read(IO_LYC) {
                io_ports.or(IO_STAT, STAT_LYC_SET);
            } else {
                io_ports.and(IO_STAT, !STAT_LYC_SET);
            }
        }
        
        // VBlank

        io_ports.and(IO_STAT, !STAT_MODE);
        io_ports.or(IO_STAT, STAT_MODE_VBLANK);
        let int_on_vblank = io_ports.read(IO_IE) & INT_VBLANK > 0;
        let int_on_m01 = (io_ports.read(IO_IE) & INT_LCDC > 0) && (io_ports.read(IO_STAT) & STAT_INT_M01 > 0);
        if ppu.ime.load(Ordering::Relaxed) && (int_on_vblank || int_on_m01) {
            if int_on_vblank { io_ports.or(IO_IF, INT_VBLANK); } else { io_ports.or(IO_IF, INT_LCDC); }
            let mut interrupted = mutex.lock().unwrap();
            *interrupted = true;
            cvar.notify_one();
        }
        for _ in 0..10 {
            wait_if_debug_break(ppu);
            sleep_precise(LINE_TIME);
            io_ports.add(IO_LY, 1);
        }

        let elapsed = frame_start.elapsed();
        let expected = FRAME_TIME;
        debug_info.actual_time_micros.store(elapsed.as_micros() as u64, Ordering::Relaxed);
        debug_info.expected_time_micros.store(expected.as_micros() as u64, Ordering::Relaxed);
    }
}

fn wait_if_debug_break(ppu: &Ppu) {
    while ppu.step_mode.load(Ordering::Acquire) {
        thread::park();
    }
}
