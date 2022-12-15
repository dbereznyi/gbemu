use std::num::ParseIntError;
use std::ops::Range;
use std::collections::HashMap;
use std::sync::atomic::{Ordering};
use crate::gameboy::gameboy::{*};
use crate::gameboy::cpu::{decode};

const CMD_HELP: &'static [(&'static str, &'static str, &'static str)] = &[
    ("step", "<enter>", "Run this instruction and break on next instruction"),
    ("over", "o", "Run this instruction and break on next instruction, stepping over function calls"),
    ("breaklist", "bl", "List all breakpoints"),
    ("breakclear", "bc", "Clear all breakpoints"),
    ("breakadd", "ba $0123", "Add a new breakpoint"),
    ("breakdel", "bd $0123", "Delete an existing breakpoint"),
    ("stackbase", "sb $0123", "Set the address to use as the base of the stack. Used by commands like stackviewall. $fffe by default."),
    ("stackview", "sv 3", "View N bytes on the stack"),
    ("stackviewall", "sva", "View all bytes on the stack"),
    ("help", "h", "Display this message"),
    ("continue", "c", "Continue execution until next breakpoint"),
    ("registers", "r", "Display an overview of current Gameboy register values"),
    ("view", "v $0123, v $0123-$0133, v $0123+$10, v $0123+16", "View contents of a range of memory addresses, or view N bytes starting from a specific address"),
    ("disassemble", "d $0123, v $0123-$0133, v $0123+$10, v $0123+16", "Disassemble the data at the given address(es)"),
];

pub enum DebugCmd {
    Step,
    Over,
    BreakList,
    BreakClear,
    BreakAdd(u16),
    BreakDel(u16),
    StackBase(u16),
    StackView(u16),
    StackViewAll,
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
        if cmd == "o" {
            return Ok(DebugCmd::Over);
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
        if cmd == "bl" {
            return Ok(DebugCmd::BreakList);
        }
        if cmd.starts_with("ba ") {
            let args = cmd.splitn(2, " ").collect::<Vec<&str>>()[1];
            let addr = parse_num(args)?;
            return Ok(DebugCmd::BreakAdd(addr));
        }
        if cmd.starts_with("bd ") {
            let args = cmd.splitn(2, " ").collect::<Vec<&str>>()[1];
            let addr = parse_num(args)?;
            return Ok(DebugCmd::BreakDel(addr));
        }
        if cmd.starts_with("sb ") {
            let args = cmd.splitn(2, " ").collect::<Vec<&str>>()[1];
            let addr = parse_num(args)?;
            return Ok(DebugCmd::StackBase(addr));
        }
        if cmd.starts_with("sv ") {
            let args = cmd.splitn(2, " ").collect::<Vec<&str>>()[1];
            let n = parse_num(args)?;
            return Ok(DebugCmd::StackView(n));
        }
        if cmd == "sva" {
            return Ok(DebugCmd::StackViewAll);
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

    pub fn run(&self, gb: &mut Gameboy) -> Result<bool, String> {
        match self {
            DebugCmd::Step => Ok(true),
            DebugCmd::Over => {
                let next_instr = decode(gb, gb.pc)
                    .map_err(|e| format!("Failed to decode instruction at ${:0>4x}: {e}", gb.pc))?;
                let addr = gb.pc + next_instr.size(gb).1;
                gb.debug.over_ret_addr = addr;
                gb.debug.step_mode.store(false, Ordering::Release);
                Ok(true)
            },
            DebugCmd::BreakList => {
                for breakpoint in gb.debug.breakpoints.iter() {
                    print!("${breakpoint:0>4x} ");
                }
                println!();
                Ok(false)
            },
            DebugCmd::BreakClear => {
                gb.debug.breakpoints.clear();
                println!("Cleared all breakpoints");
                Ok(false)
            },
            DebugCmd::BreakAdd(addr) => {
                if gb.debug.breakpoints.contains(addr) {
                    return Err("Breakpoint already exists".to_string())
                }
                gb.debug.breakpoints.push(*addr);
                println!("Added breakpoint ${addr:0>4x}");
                Ok(false)
            },
            DebugCmd::BreakDel(addr) => {
                let pos = gb.debug.breakpoints.iter().position(|&x| x == *addr)
                    .ok_or("Breakpoint does not exist".to_string())?;
                gb.debug.breakpoints.swap_remove(pos);
                println!("Deleted breakpoint ${addr:0>4x}");
                Ok(false)
            },
            DebugCmd::StackBase(addr) => {
                gb.debug.stack_base = *addr;
                println!("Set stack base to ${addr:0>4x}");
                Ok(false)
            },
            DebugCmd::StackView(n) => DebugCmd::View(gb.sp..(gb.sp+n)).run(gb),
            DebugCmd::StackViewAll => DebugCmd::View(gb.sp..gb.debug.stack_base).run(gb),
            DebugCmd::Help => {
                for &(name, usages, description) in CMD_HELP.iter() {
                    println!("{name}: {usages}\n    {description}");
                }
                println!();
                Ok(false)
            },
            DebugCmd::Continue => {
                gb.debug.step_mode.store(false, Ordering::Release);
                Ok(true)
            },
            DebugCmd::Registers => {
                println!("{}\n", gb);
                Ok(false)
            },
            DebugCmd::View(range) => {
                for addr in range.start..range.end {
                    println!("${addr:0>4X}: ${:0>2X}", gb.read(addr));
                }
                println!();
                Ok(false)
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
                Ok(false)
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
