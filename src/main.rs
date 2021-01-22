mod gameboy;

//use std::{fmt::Write, num::ParseIntError};
use std::io::{self, Write};
use std::collections::HashMap;
use crate::gameboy::{Gameboy, step};

fn main() -> std::io::Result<()> {
    let mut gb = Gameboy::new();
    
    let program = vec!(
        // ("LD B, 0xEE", vec!(0x06, 0xee)),
        // ("LD C, B", vec!(0x48)),
        // ("NOP", vec!(0x00)),
        // ("PUSH BC", vec!(0xc5)),
        // ("POP DE", vec!(0xd1)),
        // ("ADD HL, DE", vec!(0x19)),
        // ("LD A, 0x01", vec!(0x3e, 0x01)),
        // ("SUB 0x01", vec!(0xd6, 0x01)),
        // ("JP 0x0101", vec!(0xc3, 0x01, 0x01)),
        ("LD A, 0xBB", vec!(0x3e, 0xbb)),
        ("RLCA", vec!(0x07)),
        ("LD A, 0xBB", vec!(0x3e, 0xbb)),
        ("RLA", vec!(0x17)),
        ("LD A, 0x77", vec!(0x3e, 0x77)),
        ("RLA", vec!(0x17)),
    );

    run_test_program(&mut gb, program);
    Ok(())
}

fn run_test_program(gb: &mut Gameboy, program: Vec<(&str, Vec<u8>)>) {
    let mut addr_to_mnemonic = HashMap::new();

    // Load program
    let mut addr = 0x0100;
    for (mnemonic, bytes) in program.iter() {
        addr_to_mnemonic.insert(addr, *mnemonic);
        for byte in bytes.iter() {
            gb.mem[addr] = *byte;
            addr += 1;
        }
    }
    let program_end = addr as u16;

    println!("{:?}", addr_to_mnemonic);
    
    // Execute program
    println!("==> initial state\n{}\n", gb);
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    while gb.pc < program_end {
        let mnemonic = addr_to_mnemonic.get(&(gb.pc as usize)).unwrap();
        loop {
            print!("==> {}\n", mnemonic);
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
    View(usize, usize),
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

    fn run(gb: &mut Gameboy, mnemonic: &HashMap<usize, &str>, cmd: &DebugCmd) {
        match *cmd {
            DebugCmd::View(start, end) => {
                let mut addr = start;
                while addr <= end {
                    println!("${:0>4X}: ${:0>2X} {}", 
                        addr, gb.mem[addr], mnemonic.get(&addr).unwrap_or(&""));
                    addr += 1;
                }
            },
            DebugCmd::Registers => println!("{}", gb),
            _ => panic!("Invalid command {:?}", *cmd),
        }
    }
}

fn parse_num(string: &str) -> Option<usize> {
    if string.starts_with("$") {
        usize::from_str_radix(&string[1..], 16).ok()
    } else { 
        usize::from_str_radix(&string, 10).ok()
    }
}