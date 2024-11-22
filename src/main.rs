use std::env;

mod parser;
use parser::{ParseResult, ProgramInput};
mod splimer;
use splimer::Splimer;

fn main() {
    let args: Vec<String> = env::args().collect();
    dbg!(&args);

    let program_input = ProgramInput::parse(&args);

    match program_input {
        ParseResult::ThereIsNoInputFilename => {
            eprintln!("There is no input filename in arguments!");
            return;
        },
        ParseResult::MemoryValueCannotBeParsed(string) => {
            eprintln!("This string \"{}\" cannot be parsed as memory value", string);
            return;
        },
        _ => { }
    }

    let ParseResult::Success(program_input) = program_input else { panic!(); };

    let mut splimer = Splimer{program_input};

    splimer.work();
}