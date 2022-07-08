use std::num::ParseIntError;
use std::collections::HashMap;
use crate::gameboy::gameboy::{*};

#[derive(Debug)]
pub enum DebugCmd {
    Step,
    Continue,
    Registers,
    View(u16, u16),
}

impl DebugCmd {
    pub fn new(cmd: &str) -> Result<DebugCmd, String> {
        let cmd = cmd.trim();
        if cmd == "" {
            return Ok(DebugCmd::Step);
        }
        if cmd == "c" {
            return Ok(DebugCmd::Continue);
        }
        if cmd == "r" {
            return Ok(DebugCmd::Registers);
        }
        if cmd.starts_with("v ") || cmd.starts_with("view ") {
            let args = cmd.splitn(2, " ").collect::<Vec<&str>>()[1];
            if args.contains("+") {
                let args: Vec<&str> = args.split("+").collect();
                let start = parse_num(&args[0])?;
                let offset = parse_num(&args[1])?;
                return Ok(DebugCmd::View(start, start + offset));
            } else if args.contains("-") {
                let args: Vec<&str> = args.split("-").collect();
                let start = parse_num(&args[0])?;
                let end = parse_num(&args[1])?;
                return Ok(DebugCmd::View(start, end));
            } else {
                let start = parse_num(&args)?;
                return Ok(DebugCmd::View(start, start));
            }
        }
        Err(String::from("Unknown command"))
    }

    pub fn run(&self, gb: &mut Gameboy) -> bool {
        match *self {
            DebugCmd::Step => true,
            DebugCmd::Continue => {
                gb.step_mode = false;
                true
            },
            DebugCmd::View(start, end) => {
                let mut addr = start;
                while addr <= end {
                    println!("${:0>4X}: ${:0>2X}", addr, gb.read(addr));
                    addr += 1;
                }
                false
            },
            DebugCmd::Registers => {
                println!("{}\n", gb);
                false
            },
        }
    }
}

fn parse_num(string: &str) -> Result<u16, String> {
    if string.starts_with("$") {
        u16::from_str_radix(&string[1..], 16).map_err(|e| e.to_string())
    } else {
        u16::from_str_radix(&string, 10).map_err(|e| e.to_string())
    }
}
