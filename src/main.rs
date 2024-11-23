use std::env;

mod parser;
use parser::{ParseResult, ProgramInput};
mod splimer;
use splimer::Splimer;

fn main() {
    let args: Vec<String> = env::args().collect();

    let program_input = ProgramInput::parse(&args);

    match program_input {
        ParseResult::Success(_) => { }
        ParseResult::ThereIsNoInputFilename => {
            eprintln!("There is no input filename in arguments!");
            return;
        },
        ParseResult::MemoryValueCannotBeParsed(string) => {
            eprintln!("This string \"{}\" cannot be parsed as memory value", string);
            return;
        },
        _ => { panic!("Unexpected parse result") }
    }

    let ParseResult::Success(program_input) = program_input else { panic!(); };

    let mut splimer = Splimer::new(program_input);

    if splimer.program_input.to_split {
        splimer.split();
    } else {
        splimer.merge();
    }
}