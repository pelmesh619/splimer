use std::env;
use std::fs;
use std::path::Path;

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
            eprintln!("There is no input filename in arguments! \n\nUse `-h` flag to know about my arguments");
            return;
        },
        ParseResult::MemoryValueCannotBeParsed(string) => {
            eprintln!("Fragment size \"{}\" cannot be parsed as memory value \n\nUse `-h` flag to know about my arguments", string);
            return;
        },
        ParseResult::FragmentSizeIsToSmall(n) => {
            eprintln!("Fragment size should be at least {} bytes \n\nUse `-h` flag to know about my arguments", n);
            return;
        },
        ParseResult::NumberOfPartsCannotBeParsed(n) => {
            eprintln!("Number of parts \"{}\" cannot be parsed \n\nUse `-h` flag to know about my arguments", n);
            return;
        },
        ParseResult::NumberOfPartsShouldBeMoreThanOne(_) => {
            eprintln!("Number of parts should be at least 2 \n\nUse `-h` flag to know about my arguments");
            return;
        },
        ParseResult::ThereIsNoValue(string) => {
            eprintln!("For argument `{}` value is empty \n\nUse `-h` flag to know about my arguments", string);
            return;
        },
        ParseResult::Help => {
            println!(
                "
Ok, this is how to use this piece of code:

splimer
    (input_filename)                Input file name

    -S (memory-value)
    --fragment-size=(memory-value)  Size of one output fragment; can be float number
                                    with suffixes `b`, `m`, `k`, `g`,
                                    ex. `1m` or `1mb` is 1048576 bytes, 
                                    (by default is `1g`, 1073741824 bytes)

    -n (number)
    --parts=(number)                Number of output parts; should be more than 1.
                                    Makes all output files equal size.
                                    If entered `--fragment-size` will be ignored

    -s
    --split                         Splits file `input_filename`
                                    If file has `filename.ext` pattern there will be created
                                    `filename_[N].splm` files in `output_directory`
                                    (by default is true)

    -m
    --merge                         Merges files. For `input_filename` having `filename.ext` pattern
                                    program will search `filename_[N].splm` files in directory
                                    of `input_filename` and will try to merge them into `filename_[merged].ext`
                                    (by default false, ignores -n and -S arguments)

    -o (output_directory)
    --output-directory=(output_directory)   Output directory
                                            (by default it is directory, where input file lies)

    -h 
    --help                                  Show help message"
            );
            return;
        },
        _ => { panic!("Unexpected parse result") }
    }

    let ParseResult::Success(program_input) = program_input else { panic!(); };
    if let Some(dir) = &program_input.output_directory {
        fs::create_dir_all(Path::new(dir)).unwrap();
    }

    let mut splimer = Splimer::new(program_input);

    if splimer.program_input.to_split {
        splimer.split();
    } else {
        splimer.merge();
    }
}