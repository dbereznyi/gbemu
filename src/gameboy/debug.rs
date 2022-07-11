use std::num::ParseIntError;
use std::ops::Range;
use std::collections::HashMap;
use crate::gameboy::gameboy::{*};
use crate::gameboy::cpu::{decode};

const CMD_HELP: &'static [(&'static str, &'static str, &'static str)] = &[
    ("step", "<enter>", "Run this instruction and break immediately on next instruction"),
    ("help", "h", "Display this message"),
    ("continue", "c", "Run this instruction and break immediately on next instruction"),
    ("registers", "r", "Display an overview of current Gameboy register values"),
    ("view", "v $0123, v $0123-$0133, v $0123+$10, v $0123+16", "View contents of a range of memory addresses, or view N bytes starting from a specific address"),
    ("disassemble", "d $0123, v $0123-$0133, v $0123+$10, v $0123+16", "Disassemble the data at the given address(es) into instructions"),
];

pub enum DebugCmd {
    Step,
    Help,
    Continue,
    Registers,
    View(Range<u16>),
    Disassemble(Range<u16>),
}

impl DebugCmd {
    pub fn new(cmd: &str) -> Result<DebugCmd, String> {
        let cmd = cmd.trim();
        if cmd == "" {
            return Ok(DebugCmd::Step);
        }
        if cmd == "h" {
            return Ok(DebugCmd::Help);
        }
        if cmd == "c" {
            return Ok(DebugCmd::Continue);
        }
        if cmd == "r" {
            return Ok(DebugCmd::Registers);
        }
        if cmd.starts_with("v ") {
            let args = cmd.splitn(2, " ").collect::<Vec<&str>>()[1];
            let range = parse_addr_range(args)?;
            return Ok(DebugCmd::View(range));
        }
        if cmd.starts_with("d ") {
            let args = cmd.splitn(2, " ").collect::<Vec<&str>>()[1];
            let range = parse_addr_range(args)?;
            return Ok(DebugCmd::Disassemble(range));
        }
        Err(String::from("Unknown command"))
    }

    pub fn run(&self, gb: &mut Gameboy) -> bool {
        match self {
            DebugCmd::Step => true,
            DebugCmd::Help => {
                for &(name, usages, description) in CMD_HELP.iter() {
                    println!("{name}: {usages}\n    {description}");
                }
                println!();
                false
            },
            DebugCmd::Continue => {
                gb.step_mode = false;
                true
            },
            DebugCmd::Registers => {
                println!("{}\n", gb);
                false
            },
            DebugCmd::View(range) => {
                for addr in range.start..range.end {
                    println!("${addr:0>4X}: ${:0>2X}", gb.read(addr));
                }
                println!();
                false
            },
            DebugCmd::Disassemble(range) => {
                for addr in range.start..range.end {
                    println!("${addr:0>4X}: ${:0>2X} {}",
                             gb.read(addr),
                             decode(gb, addr)
                                .map(|i| i.to_string())
                                .unwrap_or("(could not disassemble)".to_string()));
                }
                println!();
                false
            },
        }
    }
}

pub fn parse_addr_range(s: &str) -> Result<Range<u16>, String> {
    if s.contains("+") {
        let args: Vec<&str> = s.split("+").collect();
        let start = parse_num(&args[0])?;
        let offset = parse_num(&args[1])?;
        Ok(Range { start, end: start + offset })
    } else if s.contains("-") {
        let args: Vec<&str> = s.split("-").collect();
        let start = parse_num(&args[0])?;
        let end = parse_num(&args[1])?;
        Ok(Range { start, end })
    } else {
        let start = parse_num(&s)?;
        Ok(Range { start: start, end: start + 1 })
    }
}

fn parse_num(string: &str) -> Result<u16, String> {
    if string.starts_with("$") {
        u16::from_str_radix(&string[1..], 16).map_err(|e| e.to_string())
    } else {
        u16::from_str_radix(&string, 10).map_err(|e| e.to_string())
    }
}
