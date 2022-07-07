mod gameboy;

extern crate sdl2;

use std::thread;
use std::time::{Duration};
use std::fs;
use std::num::{Wrapping};
use std::sync::{Arc};
use std::sync::atomic::{Ordering};
use sdl2::event::{Event};
use sdl2::keyboard::{Keycode};
use sdl2::pixels::{PixelFormatEnum};
use argparse::{ArgumentParser, Store, StoreTrue};
use crate::gameboy::{*};

struct Config {
    pub palette: [(u8,u8,u8); 4],
    pub debug_show_speed: bool,
}

impl Config {
    pub fn new() -> Result<Self, i32> {
        let mut palette_str = String::from("grey");
        let mut debug_show_speed = false;

        {
            let mut ap = ArgumentParser::new();
            ap.refer(&mut palette_str)
                .add_option(&["-p", "--palette"], Store, "Configure color palette");
            ap.refer(&mut debug_show_speed)
                .add_option(&["-s", "--debug-speed"], StoreTrue, "Write CPU and PPU speed to console");
            ap.parse_args()?;
        }

        let palette = match palette_str.as_str() {
            "grey" => PALETTE_GREY,
            "red" => PALETTE_RED,
            "green" => PALETTE_GREEN,
            "blue" => PALETTE_BLUE,
            _ => {
                println!("Unknown palette '{}', defaulting to palette 'grey'.", palette_str);
                PALETTE_GREY
            },
        };

        let config = Self {
            palette,
            debug_show_speed,
        };

        Ok(config)
    }
}

fn main() -> Result<(), String> {
    let config = Config::new()
        .map_err(|e| e.to_string())?;
    let cart_bytes = fs::read("roms/hello-world.gb")
        .expect("Failed to open ROM file");
    let cart = load_cartridge(&cart_bytes)
        .expect("Failed to parse ROM file");
    run_gameboy(cart, config)
}

fn run_gameboy(cartridge: Cartridge, config: Config) -> Result<(), String> {
    let mut gb = Gameboy::new(cartridge);

    //load_test_data(&mut gb);

    let io_ports_sdl = gb.io_ports.clone();
    let screen_sdl = gb.screen.clone();

    let debug = Debug::new();
    let cpu_expected_time_micros = debug.cpu_expected_time_micros.clone();
    let cpu_actual_time_micros = debug.cpu_actual_time_micros.clone();
    let ppu_expected_time_micros = debug.ppu_expected_time_micros.clone();
    let ppu_actual_time_micros = debug.ppu_actual_time_micros.clone();

    let mut ppu = Ppu {
        vram: gb.vram.clone(),
        oam: gb.oam.clone(),
        io_ports: gb.io_ports.clone(),
        screen: gb.screen.clone(),
        ime: gb.ime.clone(),
        interrupt_received: Arc::clone(&gb.interrupt_received),
        palette: config.palette,
    };
    thread::Builder::new().name("ppu".into()).spawn(move || { 
        run_ppu(&mut ppu, ppu_expected_time_micros, ppu_actual_time_micros);
    }).expect("Failed to create ppu thread");

    thread::Builder::new().name("cpu".into()).spawn(move || {
        run_cpu(&mut gb, cpu_expected_time_micros, cpu_actual_time_micros);
    }).expect("Failed to create cpu thread");

    // SDL code

    const SCALE: u32 = 4;

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let window = video_subsystem
        .window("gameboy emulator", 160 * SCALE, 144 * SCALE)
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
                    let scx = io_ports_sdl.read(IO_SCX);
                    let scy = io_ports_sdl.read(IO_SCY);
                    match kc {
                        Keycode::Right => io_ports_sdl.write(IO_SCX, (Wrapping(scx) - Wrapping(1)).0),
                        Keycode::Left => io_ports_sdl.write(IO_SCX, (Wrapping(scx) + Wrapping(1)).0),
                        Keycode::Up => io_ports_sdl.write(IO_SCY, (Wrapping(scy) + Wrapping(1)).0),
                        Keycode::Down => io_ports_sdl.write(IO_SCY, (Wrapping(scy) - Wrapping(1)).0),
                        Keycode::L => io_ports_sdl.xor(IO_LCDC, LCDC_ON), // Toggle LDC on/off
                        Keycode::S => io_ports_sdl.xor(IO_LCDC, LCDC_OBJ_DISP), // Toggle sprites
                        Keycode::B => io_ports_sdl.xor(IO_LCDC, LCDC_BG_DISP), // Toggle background
                        Keycode::W => io_ports_sdl.xor(IO_LCDC, LCDC_WIN_DISP), // Toggle background
                        Keycode::Q | Keycode::Escape => break 'running,
                        _ => {}
                    };
                },
                _ => {}
            }
        }

        texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
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
            let cpu_expected = debug.cpu_expected_time_micros.load(Ordering::Relaxed);
            let cpu_actual = debug.cpu_actual_time_micros.load(Ordering::Relaxed);
            let ppu_expected = debug.ppu_expected_time_micros.load(Ordering::Relaxed);
            let ppu_actual = debug.ppu_actual_time_micros.load(Ordering::Relaxed);
            println!("CPU: {}/{} ({:.4}%)", cpu_actual, cpu_expected, (cpu_actual as f64 / cpu_expected as f64) * 100.0);
            println!("PPU: {}/{} ({:.4}%)", ppu_actual, ppu_expected, (ppu_actual as f64 / ppu_expected as f64) * 100.0);
        }

        frames += 1;

        thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }

    Ok(())
}

//fn load_test_data(gb: &mut Gameboy) {
//    let mut io_ports = gb.io_ports.lock().unwrap();
//    io_ports.read(IO_LCDC) = 
//        LCDC_ON 
//        //| LCDC_WIN_DISP 
//        | LCDC_TILE_DATA 
//        | LCDC_BG_TILE_MAP 
//        //| LCDC_OBJ_SIZE
//        | LCDC_OBJ_DISP 
//        //| LCDC_BG_DISP
//        ;
//    io_ports.read(IO_BGP) = 0b1110_0100;
//    io_ports.read(IO_OBP0) = 0b1110_0100;
//    io_ports.read(IO_OBP1) = 0b1101_0000; // black, light grey, white, transparent
//    io_ports.read(IO_WX) = 7;
//    io_ports.read(IO_WY) = 136;
//
//    // init VRAM with some test data
//
//    let mut vram = gb.vram.lock().unwrap();
//    let tile_bytes = vec!(
//        // tile #0 - capital letter 'A' with some shading
//        0x7c, 0x7c, 0x00, 0xc6, 0xc6, 0x00, 0x00, 0xfe, 0xc6, 0xc6, 0x00, 0xc6, 0xc6, 0x00, 0x00, 0x00,
//        // tile #1 - dark-grey square with a 1px black border
//        0xff, 0xff, 0x81, 0xff, 0x81, 0xff, 0x81, 0xff, 0x81, 0xff, 0x81, 0xff, 0x81, 0xff, 0xff, 0xff,
//        // tile #2 - light-grey capital letter 'T'
//        0x00, 0x00, 0x00, 0x7e, 0x00, 0x7e, 0x18, 0x00, 0x18, 0x00, 0x18, 0x00, 0x18, 0x00, 0x00, 0x00,
//        // tile #3 - black arrow pointing right
//        0x10, 0x10, 0x18, 0x18, 0x1e, 0x1e, 0xff, 0xff, 0xff, 0xff, 0x1e, 0x1e, 0x18, 0x18, 0x10, 0x10,
//        // tile #4 - voiced marks
//        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x04, 0x12, 0x12, 0x08, 0x08,
//        // tile #5 - hiragana 'と'
//        0x20, 0x20, 0x20, 0x20, 0x2c, 0x2c, 0x30, 0x30, 0x40, 0x40, 0x80, 0x80, 0x80, 0x80, 0x7e, 0x7e,
//        // tile #6 - hiragana 'う'
//        0x78, 0x78, 0x00, 0x00, 0x38, 0x38, 0x44, 0x44, 0x04, 0x04, 0x04, 0x04, 0x08, 0x08, 0x30, 0x30,
//        // tile #7 - a solid white (not transparent) disk with a light-gray and black outline
//        // use with OBP1
//        0x18, 0x3c, 0x24, 0x7e, 0x5a, 0xe7, 0xbd, 0xc3, 0xbd, 0xc3, 0x5a, 0xe7, 0x24, 0x7e, 0x18, 0x3c,
//        // tile #8 - kanji '匹'
//        0xff, 0xff, 0xa4, 0xa4, 0xa4, 0xa4, 0xa5, 0xa5, 0xa3, 0xa3, 0xc0, 0xc0, 0x80, 0x80, 0xff, 0xff,
//        // tile #9 - kanji '一'
//        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfe, 0xfe, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
//        // tile #10 - kanji '二'
//        0x00, 0x00, 0x00, 0x00, 0x7c, 0x7c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xfe, 0xfe, 0x00, 0x00,
//        // tile #11 - kanji '三'
//        0x00, 0x00, 0x7c, 0x7c, 0x00, 0x00, 0x00, 0x00, 0x30, 0x30, 0x00, 0x00, 0x00, 0x00, 0xfe, 0xfe,
//        // tile #12 - kanji '四'
//        0xfe, 0xfe, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xaa, 0xae, 0xae, 0xc2, 0xc2, 0x82, 0x82, 0xfe, 0xfe,
//        // tile #13 - kanji '五'
//        0xfe, 0xfe, 0x10, 0x10, 0x10, 0x10, 0x7c, 0x7c, 0x24, 0x24, 0x44, 0x44, 0x44, 0x44, 0xfe, 0xfe,
//        // tile #14 - kanji '六'
//        0x10, 0x10, 0xfe, 0xfe, 0x00, 0x00, 0x28, 0x28, 0x6c, 0x6c, 0x44, 0x44, 0xc6, 0xc6, 0x82, 0x82,
//        // tile #15 - kanji '七'
//        0x40, 0x40, 0x46, 0x46, 0x58, 0x58, 0x60, 0x60, 0xc0, 0xc0, 0x42, 0x42, 0x42, 0x42, 0x7c, 0x7c,
//        // tile #16 - kanji '八'
//        0x38, 0x38, 0x08, 0x08, 0x2c, 0x2c, 0x24, 0x24, 0x66, 0x66, 0x42, 0x42, 0xc2, 0xc2, 0x82, 0x82,
//        // tile #17 - kanji '九'
//        0x20, 0x20, 0x20, 0x20, 0xf8, 0xf8, 0x28, 0x28, 0x28, 0x28, 0x2a, 0x2a, 0x6a, 0x6a, 0xce, 0xce,
//        // tile #18 - number '2'
//        0x00, 0x00, 0x00, 0x00, 0x7c, 0x7c, 0xc6, 0xc6, 0x0e, 0x0e, 0x78, 0x78, 0xe0, 0xe0, 0xfe, 0xfe,
//    );
//    for (i, byte) in tile_bytes.iter().enumerate() {
//        vram[i] = *byte;
//    }
//
//    let bg_tile_map_start = 
//        if io_ports.read(IO_LCDC) & LCDC_BG_TILE_MAP > 0 {
//            0x1c00
//        } else {
//            0x1800
//        };
//    let win_tile_map_start =
//        if io_ports.read(IO_LCDC) & LCDC_WIN_TILE_MAP > 0 {
//            0x1c00
//        } else {
//            0x1800
//        };
//
//    for i in 0..32*32 {
//        vram[bg_tile_map_start + i] = 0;
//        vram[win_tile_map_start + i] = 1;
//    }
//
//    let oam_bytes = vec!(
//        // sprite #0
//        // y=24, x=8, tile #9, no flags
//        24, 8, 0x09, 0b0000_0000,
//        // sprite #1
//        // y=24, x=16, tile #10, no flags
//        24, 16, 0x0a, 0b0000_0000,
//        // sprite #2
//        // y=24, x=24, tile #11, no flags
//        24, 24, 0x0b, 0b0000_0000,
//        // sprite #3
//        // y=24, x=32, tile #12, no flags
//        24, 32, 0x0c, 0b0000_0000,
//        // sprite #4
//        // y=24, x=40, tile #13, no flags
//        24, 40, 0x0d, 0b0000_0000,
//        // sprite #5
//        // y=24, x=48, tile #14, no flags
//        24, 48, 0x0e, 0b0000_0000,
//        // sprite #6
//        // y=24, x=56, tile #15, no flags
//        24, 56, 0x0f, 0b0000_0000,
//        // sprite #7
//        // y=24, x=64, tile #16, no flags
//        24, 64, 0x10, 0b0000_0000,
//        // sprite #8
//        // y=24, x=72, tile #17, no flags
//        24, 72, 0x11, 0b0000_0000,
//        // sprite #9
//        // y=40, x=64, tile #11, no flags
//        40, 64, 0x0b, 0b0000_0000,
//        // sprite #10
//        // y=40, x=72, tile #8, no flags
//        40, 72, 0x08, 0b0000_0000,
//        // sprite #11
//        // y=40, x=8, tile #18, no flags
//        40, 8, 0x12, 0b0000_0000,
//    );
//    let mut oam = gb.oam.lock().unwrap();
//    for (i, byte) in oam_bytes.iter().enumerate() {
//        oam[i] = *byte;
//    }
//
//    gb.halted.store(true, Ordering::Relaxed);
//}

//fn run_test_program(gb: &mut Gameboy, program: Vec<(&str, Vec<u8>)>) {
//    let mut addr_to_mnemonic = HashMap::new();
//
//    // Load program
//    let mut addr = 0x0100;
//    for (mnemonic, bytes) in program.iter() {
//        addr_to_mnemonic.insert(addr, *mnemonic);
//        for byte in bytes.iter() {
//            gb.write(addr, *byte);
//            addr += 1;
//        }
//    }
//    let program_end = addr;
//
//    println!("{:?}", addr_to_mnemonic);
//    
//    // Execute program
//    println!("==> initial state\n{}\n", gb);
//    let stdin = io::stdin();
//    let mut stdout = io::stdout();
//    while gb.pc < program_end {
//        let mnemonic = addr_to_mnemonic.get(&gb.pc).unwrap();
//        loop {
//            print!("BREAK **** {}\n", mnemonic);
//            print!("> ");
//            stdout.flush().unwrap();
//            let mut line = String::new();
//            stdin.read_line(&mut line).unwrap();
//            let cmd = DebugCmd::new(&line);
//            match cmd {
//                Result::Ok(cmd) => {
//                    if let DebugCmd::Step = cmd {
//                        break;
//                    }
//                    DebugCmd::run(gb, &addr_to_mnemonic, &cmd)
//                },
//                Result::Err(err) => println!("{}", err),
//            }
//        }
//        step(gb);
//    }
//}
//
//#[derive(Debug)]
//enum DebugCmd {
//    Step,
//    Registers,
//    View(u16, u16),
//}
//
//impl DebugCmd {
//    fn new(cmd: &str) -> Result<DebugCmd, &str> {
//        let cmd = cmd.trim();
//        if cmd == "" {
//            return Result::Ok(DebugCmd::Step);
//        }
//        if cmd == "r" {
//            return Result::Ok(DebugCmd::Registers);
//        }
//        if cmd.starts_with("v ") || cmd.starts_with("view ") {
//            let args = cmd.splitn(2, " ").collect::<Vec<&str>>()[1];
//            if args.contains("+") {
//                let args: Vec<&str> = args.split("+").collect();
//                let start = parse_num(&args[0]).expect("Failed to parse start address");
//                let offset = parse_num(&args[1]).expect("Failed to parse offset");
//                return Result::Ok(DebugCmd::View(start, start + offset));
//            } else if args.contains("-") {
//                let args: Vec<&str> = args.split("-").collect();
//                let start = parse_num(&args[0]).expect("Failed to parse start address");
//                let end = parse_num(&args[1]).expect("Failed to parse end address");
//                return Result::Ok(DebugCmd::View(start, end));
//            } else {
//                let start = parse_num(&args).expect("Failed to parse start address");
//                return Result::Ok(DebugCmd::View(start, start));
//            }
//        }
//        Result::Err("Unknown command")
//    }
//
//    fn run(gb: &mut Gameboy, mnemonic: &HashMap<u16, &str>, cmd: &DebugCmd) {
//        match *cmd {
//            DebugCmd::View(start, end) => {
//                let mut addr = start;
//                while addr <= end {
//                    println!("${:0>4X}: ${:0>2X} {}", 
//                        addr, gb.read(addr), mnemonic.get(&addr).unwrap_or(&""));
//                    addr += 1;
//                }
//            },
//            DebugCmd::Registers => println!("{}", gb),
//            _ => panic!("Invalid command {:?}", *cmd),
//        }
//    }
//}
//
//fn parse_num(string: &str) -> Option<u16> {
//    if string.starts_with("$") {
//        u16::from_str_radix(&string[1..], 16).ok()
//    } else { 
//        u16::from_str_radix(&string, 10).ok()
//    }
//}
