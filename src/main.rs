mod gameboy;

extern crate sdl2;

use std::thread;
use std::time::{Duration};
use std::fs;
use std::num::{Wrapping};
use std::sync::{Arc};
use std::sync::atomic::{Ordering};
use sdl2::event::{Event};
use sdl2::keyboard::{Keycode, Scancode};
use sdl2::pixels::{PixelFormatEnum};
use argparse::{ArgumentParser, Store, StoreTrue};
use crate::gameboy::{*};

struct Config {
    pub rom_filepath: String,
    pub scale: u32,
    pub palette: [(u8,u8,u8); 4],
    pub debug_show_speed: bool,
    pub breakpoints: Vec<u16>,
}

impl Config {
    pub fn new() -> Result<Self, String> {
        let mut rom_filepath = String::from("roms/hello-world.gb");
        let mut scale = 4;
        let mut palette_str = String::from("grey");
        let mut debug_show_speed = false;
        let mut breakpoints_str = String::from("");

        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut rom_filepath)
                .add_argument("rom_filepath", Store, "Path to a Gameboy ROM file");
            ap.refer(&mut scale)
                .add_option(&["-s", "--scale"], Store, "Scale factor for the display (e.g. 1x, 2x, 3x...)");
            ap.refer(&mut palette_str)
                .add_option(&["-p", "--palette"], Store, "Configure color palette");
            ap.refer(&mut debug_show_speed)
                .add_option(&["-d", "--debug-speed"], StoreTrue, "Write CPU and PPU speed to console");
            ap.refer(&mut breakpoints_str)
                .add_option(&["-b", "--breakpoints"], Store, "List of addresses (in hexadecimal) to set as breakpoints for debugging, separated by commas");
            ap.parse_args()
                .map_err(|e| format!("Argument parsing failed with error code {e}"))?;
        }

        if scale < 1 {
            println!("Minimum allowed scale factor is 1, clamping.");
            scale = 1;
        }

        let palette = match palette_str.as_str() {
            "grey" => PALETTE_GREY,
            "red" => PALETTE_RED,
            "green" => PALETTE_GREEN,
            "blue" => PALETTE_BLUE,
            _ => {
                println!("Unknown palette '{palette_str}', defaulting to palette 'grey'.");
                PALETTE_GREY
            },
        };

        let mut breakpoints = vec!();
        for breakpoint in breakpoints_str.split(',') {
            if breakpoint.is_empty() {
                // Split returns a single empty string when the string being split is empty.
                break;
            }
            let breakpoint = breakpoint.trim();
            let breakpoint_u16 = u16::from_str_radix(breakpoint, 16)
                .map_err(|e| format!("Failed to parse breakpoint {breakpoint}: {e}"))?;
            println!("Added breakpoint ${breakpoint_u16:0>4x}");
            breakpoints.push(breakpoint_u16);
        }

        let config = Self {
            rom_filepath,
            scale,
            palette,
            debug_show_speed,
            breakpoints,
        };

        Ok(config)
    }
}

fn main() -> Result<(), String> {
    let config = Config::new()?;
    let cart_bytes = fs::read(&config.rom_filepath)
        .expect("Failed to open ROM file");
    let cart = load_cartridge(&cart_bytes)
        .expect("Failed to parse ROM file");
    run_gameboy(cart, config)
}

fn run_gameboy(cartridge: Cartridge, config: Config) -> Result<(), String> {
    let mut gb = Gameboy::new(cartridge);

    for breakpoint in &config.breakpoints {
        gb.debug.breakpoints.push(*breakpoint);
    }

    let io_ports_sdl = gb.io_ports.clone();
    let ime_sdl = gb.ime.clone();
    let controller_data_sdl = gb.controller_data.clone();
    let interrupt_received_sdl = Arc::clone(&gb.interrupt_received);
    let screen_sdl = gb.screen.clone();

    let debug_info_cpu = DebugInfoCpu::new();
    let debug_info_ppu = DebugInfoPpu::new();

    let ppu_thread = {
        let mut ppu = Ppu::new(&gb, config.palette);
        let debug = debug_info_ppu.clone();

        thread::Builder::new().name("ppu".into()).spawn(move || { 
            run_ppu(&mut ppu, debug);
        }).expect("Failed to create ppu thread")
    };

    let mut timer = Timer {
        io_ports: gb.io_ports.clone(),
        ime: gb.ime.clone(),
        interrupt_received: Arc::clone(&gb.interrupt_received),
        timer_enabled: Arc::clone(&gb.timer_enabled),
    };
    let timer_thread = thread::Builder::new().name("timer".into()).spawn(move || {
        run_timer(&mut timer);
    }).expect("Failed to create timer thread");

    {
        let debug = debug_info_cpu.clone();
        let components = vec!(ppu_thread, timer_thread);

        thread::Builder::new().name("cpu".into()).spawn(move || {
            run_cpu(&mut gb, debug, components.as_slice());
        }).expect("Failed to create cpu thread");
    }

    // SDL code

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window("gameboy emulator", 160 * config.scale, 144 * config.scale)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;

    let mut canvas = window
        .into_canvas()
        .build()
        .map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();
    
    let mut texture = texture_creator
        .create_texture_streaming(PixelFormatEnum::RGB24, 160, 144)
        .map_err(|e| e.to_string())?;

    canvas.clear();

    let mut frames: u128 = 0;
    let mut event_pump = sdl_context.event_pump()?;
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => break 'running,
                Event::KeyDown { keycode: Some(kc), .. } => {
                    match kc {
                        Keycode::L => io_ports_sdl.xor(IO_LCDC, LCDC_ON), // Toggle LDC on/off
                        Keycode::S => io_ports_sdl.xor(IO_LCDC, LCDC_OBJ_DISP), // Toggle sprites
                        Keycode::B => io_ports_sdl.xor(IO_LCDC, LCDC_BG_DISP), // Toggle background
                        Keycode::W => io_ports_sdl.xor(IO_LCDC, LCDC_WIN_DISP), // Toggle window
                        _ => {}
                    };
                },
                _ => {}
            }
        }

        let mut cont_data = 0b1111_1111;
        let kb_state = event_pump.keyboard_state();
        if kb_state.is_scancode_pressed(Scancode::Right) {
            cont_data &= !CONTROLLER_DATA_RIGHT;
        }
        if kb_state.is_scancode_pressed(Scancode::Left) {
            cont_data &= !CONTROLLER_DATA_LEFT;
        }
        if kb_state.is_scancode_pressed(Scancode::Up) {
            cont_data &= !CONTROLLER_DATA_UP;
        }
        if kb_state.is_scancode_pressed(Scancode::Down) {
            cont_data &= !CONTROLLER_DATA_DOWN;
        }
        if kb_state.is_scancode_pressed(Scancode::X) {
            cont_data &= !CONTROLLER_DATA_A;
        }
        if kb_state.is_scancode_pressed(Scancode::Z) {
            cont_data &= !CONTROLLER_DATA_B;
        }
        if kb_state.is_scancode_pressed(Scancode::A) {
            cont_data &= !CONTROLLER_DATA_SE;
        }
        if kb_state.is_scancode_pressed(Scancode::S) {
            cont_data &= !CONTROLLER_DATA_ST;
        }
        let prev_cont_data = controller_data_sdl.load(Ordering::Relaxed);
        controller_data_sdl.store(cont_data, Ordering::Relaxed);
        // Technically we should only trigger this interrupt when a low signal lasts for 2^4 *
        // 4MHz = 4 microsecs. We currently poll 60 times per second, which is once every ~16,666 microseconds.
        // If we need more sensitive polling, could move controller handling to its own thread.
        if ime_sdl.load(Ordering::Relaxed) && io_ports_sdl.read(IO_IE) & INT_HILO > 0 {
            for i in 0..8 {
                if prev_cont_data & (1 << i) > 0 && cont_data & (1 << i) == 0 {
                    io_ports_sdl.or(IO_IF, INT_HILO);
                    let (mutex, cvar) = &*interrupt_received_sdl;
                    let mut interrupted = mutex.lock().unwrap();
                    *interrupted = true;
                    cvar.notify_one();
                    break;
                }
            }
        }

        texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
            // TODO We should probably have PPU write to a backbuffer and just swap the buffers
            // here instead, could fix the uneveness in updating the screen.
            let screen = screen_sdl.lock().unwrap();
            for y in 0..144 {
                for x in 0..160 {
                    let offset = y*pitch + x*3;
                    buffer[offset] = screen[y][x].0;
                    buffer[offset + 1] = screen[y][x].1;
                    buffer[offset + 2] = screen[y][x].2;
                }
            }
        })?;
        canvas.copy(&texture, None, None)?;
        canvas.present();

        if config.debug_show_speed && frames % 30 == 0 {
            let cpu_expected = debug_info_cpu.expected_time_nanos.load(Ordering::Relaxed);
            let cpu_actual = debug_info_cpu.actual_time_nanos.load(Ordering::Relaxed);
            let ppu_expected = debug_info_ppu.expected_time_micros.load(Ordering::Relaxed);
            let ppu_actual = debug_info_ppu.actual_time_micros.load(Ordering::Relaxed);
            let oam_time = debug_info_ppu.oam_time_nanos.load(Ordering::Relaxed);
            println!("CPU: {}/{} ({:.4}%)", cpu_expected, cpu_actual, (cpu_expected as f64 / cpu_actual as f64) * 100.0);
            println!("PPU: {}/{} ({:.4}%), oam {}ns", ppu_expected, ppu_actual, (ppu_expected as f64 / ppu_actual as f64) * 100.0, oam_time);
        }

        frames += 1;

        thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }

    Ok(())
}
