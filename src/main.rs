use std::env;

struct ProgramInput {
    input_filename: String
}

impl ProgramInput {
    pub fn parse(arguments: &Vec<String>) {

    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    dbg!(&args);

    let program_input = ProgramInput::parse(&args);
}