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
    run_gameboy()
}

fn run_gameboy() -> Result<(), String> {
    let mut gb = Gameboy::new();

    {
        let mut io_ports = gb.io_ports.lock().unwrap();
        io_ports[IO_LCDC] = 
            LCDC_ON | LCDC_TILE_DATA | LCDC_BG_TILE_MAP | LCDC_OBJ_DISP | LCDC_BG_WIN_DISP; 
        io_ports[IO_BGP] = 0b1110_0100;

        // init VRAM with some test data

        let mut vram = gb.vram.lock().unwrap();
        let tile_bytes = vec!(
            0x7c, 0x7c, 
            0x00, 0xc6, 
            0xc6, 0x00, 
            0x00, 0xfe,
            0xc6, 0xc6, 
            0x00, 0xc6, 
            0xc6, 0x00,
            0x00, 0x00);
        for (i, byte) in tile_bytes.iter().enumerate() {
            vram[i] = *byte;
        }

        // Just repeat tile #0
        let mut tile_map_bytes = vec!();
        for i in 0..32*32 {
            tile_map_bytes.push(0);
        }
        for (i, byte) in tile_map_bytes.iter().enumerate() {
            vram[0x1c00 + i] = *byte;
        }

        gb.halted.store(true, Ordering::Relaxed);
    }

    let io_ports_sdl = gb.io_ports.clone();
    let screen_sdl = gb.screen.clone();

    let mut ppu = Ppu {
        vram: gb.vram.clone(),
        oam: gb.oam.clone(),
        io_ports: gb.io_ports.clone(),
        io_ie: gb.io_ie.clone(),
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
