mod gameboy;

extern crate timer;
extern crate sdl2;

use std::thread;
use std::time::{Duration, Instant};
use std::io::{self, Write};
use std::collections::HashMap;
use std::num::Wrapping;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use sdl2::event::Event;
use sdl2::keyboard::Keycode;
use sdl2::pixels::PixelFormatEnum;
use crate::gameboy::{*};

fn main() -> Result<(), String> {
    let dummy_cart_bytes: Box<[u8; 0x8000]> = Box::new([0; 0x8000]);
    let dummy_cart = load_cartridge(&*dummy_cart_bytes).unwrap();
    run_gameboy(dummy_cart)
}

fn run_gameboy(cartridge: Cartridge) -> Result<(), String> {
    let mut gb = Gameboy::new(cartridge);

    load_test_data(&mut gb);

    let io_ports_sdl = gb.io_ports.clone();
    let screen_sdl = gb.screen.clone();

    let mut ppu = Ppu {
        vram: gb.vram.clone(),
        oam: gb.oam.clone(),
        io_ports: gb.io_ports.clone(),
        screen: gb.screen.clone(),
        ime: gb.ime.clone(),
        interrupt_received: Arc::clone(&gb.interrupt_received),
    };
    thread::spawn(move || { 
        run_ppu(&mut ppu);
    });

    thread::spawn(move || {
        run_cpu(&mut gb);
    });

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

    let mut color: u32 = 0;

    // An initial test texture
    texture.with_lock(None, |buffer: &mut [u8], pitch: usize| {
        for y in 0..144 {
            for x in 0..160 {
                let offset = y*pitch + x*3;

                buffer[offset] = color as u8;
                buffer[offset + 1] = color as u8;
                buffer[offset + 2] = color as u8;

                color += 64;
                color %= 256;
            }
            color += 64;
            color %= 256;
        }
    })?;

    canvas.clear();
    canvas.copy(&texture, None, None)?;
    canvas.present();

    let mut event_pump = sdl_context.event_pump()?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } | Event::KeyDown { keycode: Some(Keycode::Escape), .. } => break 'running,
                Event::KeyDown { keycode: Some(kc), .. } => {
                    let mut io_ports = io_ports_sdl.lock().unwrap();
                    match kc {
                        Keycode::Right => io_ports[IO_SCX] = (Wrapping(io_ports[IO_SCX]) - Wrapping(1)).0,
                        Keycode::Left => io_ports[IO_SCX] = (Wrapping(io_ports[IO_SCX]) + Wrapping(1)).0,
                        Keycode::Up => io_ports[IO_SCY] = (Wrapping(io_ports[IO_SCY]) + Wrapping(1)).0,
                        Keycode::Down => io_ports[IO_SCY] = (Wrapping(io_ports[IO_SCY]) - Wrapping(1)).0,
                        Keycode::L => io_ports[IO_LCDC] ^= LCDC_ON, // Toggle LDC on/off
                        Keycode::S => io_ports[IO_LCDC] ^= LCDC_OBJ_DISP, // Toggle sprites
                        Keycode::B => io_ports[IO_LCDC] ^= LCDC_BG_DISP, // Toggle background
                        Keycode::W => io_ports[IO_LCDC] ^= LCDC_WIN_DISP, // Toggle background
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
                    buffer[offset] = screen[y][x];
                    buffer[offset + 1] = screen[y][x];
                    buffer[offset + 2] = screen[y][x];
                }
            }
        })?;
        canvas.copy(&texture, None, None)?;
        canvas.present();

        thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }

    Ok(())
}

fn run_cpu(gb: &mut Gameboy) {
    loop {
        // If CPU is halted, just wait until an interrupt wakes us up
        // CPU will only be interrupted if IME is set and corresponding IE bit is set,
        // so we will always get an interrupt we can process from this
        if gb.halted.load(Ordering::Relaxed) {
            let (mutex, cvar) = &*gb.interrupt_received;
            let mut interrupted = mutex.lock().unwrap();
            while !*interrupted {
                interrupted = cvar.wait(interrupted).unwrap();
            }
            *interrupted = false;
        } 
        
        let cycles_start = gb.cycles;
        let start = Instant::now();

        // To handle an interrupt:
        // 1. Push PC onto the stack
        // 2. Set PC to address corresponding to interrupt type
        // 3. Set corresponding bit in IF register to 0
        // 4. Disable interrupts by un-setting IME
        // From there the interrupt routine is jumped to and that code is responsible for
        // re-enabling interrupts, e.g. through RETI
        let mut io_ports = *gb.io_ports.lock().unwrap();
        if gb.ime.load(Ordering::Relaxed) && io_ports[IO_IF] > 0 {
            push_pc(gb);

            if io_ports[IO_IF] & VBLANK > 0 {
                gb.pc = 0x0040;
                io_ports[IO_IF] &= !VBLANK;
            } else if io_ports[IO_IF] & LCDC > 0 {
                gb.pc = 0x0048;
                io_ports[IO_IF] &= !LCDC;
            } else if io_ports[IO_IF] & TIMER > 0 {
                gb.pc = 0x0050;
                io_ports[IO_IF] &= !TIMER;
            } else if io_ports[IO_IF] & SERIAL > 0 {
                gb.pc = 0x0058;
                io_ports[IO_IF] &= !SERIAL;
            } else if io_ports[IO_IF] & HI_TO_LOW > 0 {
                gb.pc = 0x0060;
                io_ports[IO_IF] &= !HI_TO_LOW;
            }

            gb.ime.store(false, Ordering::Relaxed);
            let (mutex, cvar) = &*gb.interrupt_received;
            let mut interrupted = mutex.lock().unwrap();
            *interrupted = false;
        }
        drop(io_ports);

        step(gb);

        let cycles_elapsed = (gb.cycles - cycles_start) as u32;
        thread::sleep(Duration::new(0, 1000*cycles_elapsed) - start.elapsed());
    }
}

fn load_test_data(gb: &mut Gameboy) {
    let mut io_ports = gb.io_ports.lock().unwrap();
    io_ports[IO_LCDC] = 
        LCDC_ON 
        //| LCDC_WIN_DISP 
        | LCDC_TILE_DATA 
        | LCDC_BG_TILE_MAP 
        //| LCDC_OBJ_SIZE
        | LCDC_OBJ_DISP 
        //| LCDC_BG_DISP
        ;
    io_ports[IO_BGP] = 0b1110_0100;
    io_ports[IO_OBP0] = 0b1110_0100;
    io_ports[IO_OBP1] = 0b1101_0000; // black, light grey, white, transparent
    io_ports[IO_WX] = 7;
    io_ports[IO_WY] = 136;

    // init VRAM with some test data

    let mut vram = gb.vram.lock().unwrap();
    let tile_bytes = vec!(
        // tile #0 - capital letter 'A' with some shading
        0x7c, 0x7c, 
        0x00, 0xc6, 
        0xc6, 0x00, 
        0x00, 0xfe,
        0xc6, 0xc6, 
        0x00, 0xc6, 
        0xc6, 0x00,
        0x00, 0x00,
        // tile #1 - dark-grey square with a 1px black border
        0xff, 0xff,
        0x81, 0xff,
        0x81, 0xff,
        0x81, 0xff,
        0x81, 0xff,
        0x81, 0xff,
        0x81, 0xff,
        0xff, 0xff,
        // tile #2 - light-grey capital letter 'T'
        0x00, 0x00,
        0x00, 0x7e,
        0x00, 0x7e,
        0x18, 0x00,
        0x18, 0x00,
        0x18, 0x00,
        0x18, 0x00,
        0x00, 0x00,
        // tile #3 - black arrow pointing right
        0x10, 0x10,
        0x18, 0x18,
        0x1e, 0x1e,
        0xff, 0xff,
        0xff, 0xff,
        0x1e, 0x1e,
        0x18, 0x18,
        0x10, 0x10,
        // tile #4 - voiced marks
        0x00, 0x00,
        0x00, 0x00,
        0x00, 0x00,
        0x00, 0x00,
        0x00, 0x00,
        0x04, 0x04,
        0x12, 0x12,
        0x08, 0x08,
        // tile #5 - hiragana 'と'
        0x20, 0x20,
        0x20, 0x20,
        0x2c, 0x2c,
        0x30, 0x30,
        0x40, 0x40,
        0x80, 0x80,
        0x80, 0x80,
        0x7e, 0x7e,
        // tile #6 - hiragana 'う'
        0x78, 0x78,
        0x00, 0x00,
        0x38, 0x38,
        0x44, 0x44,
        0x04, 0x04,
        0x04, 0x04,
        0x08, 0x08,
        0x30, 0x30,
        // tile #7 - a solid white (not transparent) disk with a light-gray and black outline
        // use with OBP1
        0x18, 0x3c,
        0x24, 0x7e,
        0x5a, 0xe7,
        0xbd, 0xc3,
        0xbd, 0xc3,
        0x5a, 0xe7,
        0x24, 0x7e,
        0x18, 0x3c,
        // tile #8 - kanji '匹'
        0xff, 0xff,
        0xa4, 0xa4,
        0xa4, 0xa4,
        0xa5, 0xa5,
        0xa3, 0xa3,
        0xc0, 0xc0,
        0x80, 0x80,
        0xff, 0xff,
        // tile #9 - kanji '一'
        0x00, 0x00,
        0x00, 0x00,
        0x00, 0x00,
        0xfe, 0xfe,
        0x00, 0x00,
        0x00, 0x00,
        0x00, 0x00,
        0x00, 0x00,
        // tile #10 - kanji '二'
        0x00, 0x00,
        0x00, 0x00,
        0x7c, 0x7c,
        0x00, 0x00,
        0x00, 0x00,
        0x00, 0x00,
        0xfe, 0xfe,
        0x00, 0x00,
        // tile #11 - kanji '三'
        0x00, 0x00,
        0x7c, 0x7c,
        0x00, 0x00,
        0x00, 0x00,
        0x30, 0x30,
        0x00, 0x00,
        0x00, 0x00,
        0xfe, 0xfe,
        // tile #12 - kanji '四'
        0xfe, 0xfe,
        0xaa, 0xaa,
        0xaa, 0xaa,
        0xaa, 0xaa,
        0xae, 0xae,
        0xc2, 0xc2,
        0x82, 0x82,
        0xfe, 0xfe,
        // tile #13 - kanji '五'
        0xfe, 0xfe,
        0x10, 0x10,
        0x10, 0x10,
        0x7c, 0x7c,
        0x24, 0x24,
        0x44, 0x44,
        0x44, 0x44,
        0xfe, 0xfe,
        // tile #14 - kanji '六'
        0x10, 0x10,
        0xfe, 0xfe,
        0x00, 0x00,
        0x28, 0x28,
        0x6c, 0x6c,
        0x44, 0x44,
        0xc6, 0xc6,
        0x82, 0x82,
        // tile #15 - kanji '七'
        0x40, 0x40,
        0x46, 0x46,
        0x58, 0x58,
        0x60, 0x60,
        0xc0, 0xc0,
        0x42, 0x42,
        0x42, 0x42,
        0x7c, 0x7c,
        // tile #16 - kanji '八'
        0x38, 0x38,
        0x08, 0x08,
        0x2c, 0x2c,
        0x24, 0x24,
        0x66, 0x66,
        0x42, 0x42,
        0xc2, 0xc2,
        0x82, 0x82,
        // tile #17 - kanji '九'
        0x20, 0x20,
        0x20, 0x20,
        0xf8, 0xf8,
        0x28, 0x28,
        0x28, 0x28,
        0x2a, 0x2a,
        0x6a, 0x6a,
        0xce, 0xce,
        // tile #18 - number '2'
        0x00, 0x00,
        0x00, 0x00,
        0x7c, 0x7c,
        0xc6, 0xc6,
        0x0e, 0x0e,
        0x78, 0x78,
        0xe0, 0xe0,
        0xfe, 0xfe,
    );
    for (i, byte) in tile_bytes.iter().enumerate() {
        vram[i] = *byte;
    }

    let bg_tile_map_start = 
        if io_ports[IO_LCDC] & LCDC_BG_TILE_MAP > 0 {
            0x1c00
        } else {
            0x1800
        };
    let win_tile_map_start =
        if io_ports[IO_LCDC] & LCDC_WIN_TILE_MAP > 0 {
            0x1c00
        } else {
            0x1800
        };

    for i in 0..32*32 {
        vram[bg_tile_map_start + i] = 1;
        vram[win_tile_map_start + i] = 1;
    }

    let oam_bytes = vec!(
        // sprite #0
        // y=24, x=8, tile #9, no flags
        24, 8, 0x09, 0b0000_0000,
        // sprite #1
        // y=24, x=16, tile #10, no flags
        24, 16, 0x0a, 0b0000_0000,
        // sprite #2
        // y=24, x=24, tile #11, no flags
        24, 24, 0x0b, 0b0000_0000,
        // sprite #3
        // y=24, x=32, tile #12, no flags
        24, 32, 0x0c, 0b0000_0000,
        // sprite #4
        // y=24, x=40, tile #13, no flags
        24, 40, 0x0d, 0b0000_0000,
        // sprite #5
        // y=24, x=48, tile #14, no flags
        24, 48, 0x0e, 0b0000_0000,
        // sprite #6
        // y=24, x=56, tile #15, no flags
        24, 56, 0x0f, 0b0000_0000,
        // sprite #7
        // y=24, x=64, tile #16, no flags
        24, 64, 0x10, 0b0000_0000,
        // sprite #8
        // y=24, x=72, tile #17, no flags
        24, 72, 0x11, 0b0000_0000,
        // sprite #9
        // y=40, x=64, tile #11, no flags
        40, 64, 0x0b, 0b0000_0000,
        // sprite #10
        // y=40, x=72, tile #8, no flags
        40, 72, 0x08, 0b0000_0000,
        // sprite #11
        // y=40, x=8, tile #18, no flags
        40, 8, 0x12, 0b0000_0000,
    );
    let mut oam = gb.oam.lock().unwrap();
    for (i, byte) in oam_bytes.iter().enumerate() {
        oam[i] = *byte;
    }

    gb.halted.store(true, Ordering::Relaxed);
}

fn run_test_program(gb: &mut Gameboy, program: Vec<(&str, Vec<u8>)>) {
    let mut addr_to_mnemonic = HashMap::new();

    // Load program
    let mut addr = 0x0100;
    for (mnemonic, bytes) in program.iter() {
        addr_to_mnemonic.insert(addr, *mnemonic);
        for byte in bytes.iter() {
            gb.write(addr, *byte);
            addr += 1;
        }
    }
    let program_end = addr;

    println!("{:?}", addr_to_mnemonic);
    
    // Execute program
    println!("==> initial state\n{}\n", gb);
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    while gb.pc < program_end {
        let mnemonic = addr_to_mnemonic.get(&gb.pc).unwrap();
        loop {
            print!("BREAK **** {}\n", mnemonic);
            print!("> ");
            stdout.flush().unwrap();
            let mut line = String::new();
            stdin.read_line(&mut line).unwrap();
            let cmd = DebugCmd::new(&line);
            match cmd {
                Result::Ok(cmd) => {
                    if let DebugCmd::Step = cmd {
                        break;
                    }
                    DebugCmd::run(gb, &addr_to_mnemonic, &cmd)
                },
                Result::Err(err) => println!("{}", err),
            }
        }
        step(gb);
    }
}

#[derive(Debug)]
enum DebugCmd {
    Step,
    Registers,
    View(u16, u16),
}

impl DebugCmd {
    fn new(cmd: &str) -> Result<DebugCmd, &str> {
        let cmd = cmd.trim();
        if cmd == "" {
            return Result::Ok(DebugCmd::Step);
        }
        if cmd == "r" {
            return Result::Ok(DebugCmd::Registers);
        }
        if cmd.starts_with("v ") || cmd.starts_with("view ") {
            let args = cmd.splitn(2, " ").collect::<Vec<&str>>()[1];
            if args.contains("+") {
                let args: Vec<&str> = args.split("+").collect();
                let start = parse_num(&args[0]).expect("Failed to parse start address");
                let offset = parse_num(&args[1]).expect("Failed to parse offset");
                return Result::Ok(DebugCmd::View(start, start + offset));
            } else if args.contains("-") {
                let args: Vec<&str> = args.split("-").collect();
                let start = parse_num(&args[0]).expect("Failed to parse start address");
                let end = parse_num(&args[1]).expect("Failed to parse end address");
                return Result::Ok(DebugCmd::View(start, end));
            } else {
                let start = parse_num(&args).expect("Failed to parse start address");
                return Result::Ok(DebugCmd::View(start, start));
            }
        }
        Result::Err("Unknown command")
    }

    fn run(gb: &mut Gameboy, mnemonic: &HashMap<u16, &str>, cmd: &DebugCmd) {
        match *cmd {
            DebugCmd::View(start, end) => {
                let mut addr = start;
                while addr <= end {
                    println!("${:0>4X}: ${:0>2X} {}", 
                        addr, gb.read(addr), mnemonic.get(&addr).unwrap_or(&""));
                    addr += 1;
                }
            },
            DebugCmd::Registers => println!("{}", gb),
            _ => panic!("Invalid command {:?}", *cmd),
        }
    }
}

fn parse_num(string: &str) -> Option<u16> {
    if string.starts_with("$") {
        u16::from_str_radix(&string[1..], 16).ok()
    } else { 
        u16::from_str_radix(&string, 10).ok()
    }
}
