mod gameboy;

extern crate timer;
extern crate sdl2;

use std::thread;
use std::time::{Duration};
use std::io::{self, Write};
use std::collections::HashMap;
use std::num::Wrapping;
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
    }

    let io_ports_sdl = gb.io_ports.clone();
    let screen_sdl = gb.screen.clone();

    let vram = gb.vram.clone();
    let oam = gb.oam.clone();
    let io_ports = gb.io_ports.clone();
    let screen_ppu = gb.screen.clone();
    thread::spawn(move || { 
        run_ppu(vram, oam, io_ports, screen_ppu);        
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
    // TODO implement

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
