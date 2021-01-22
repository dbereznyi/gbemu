mod instruction;
mod exec;
mod tests;
mod step;

pub use step::step;
pub use instruction::Instr;
//pub use asm::assemble;