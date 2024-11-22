use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};

use crate::parser::{ParseResult, ProgramInput};

pub struct Splimer {
    pub program_input: ProgramInput
}

impl Splimer {
    pub fn work(&mut self) {
        let metadata = self.check_file_access(fs::metadata(&self.program_input.input_filename));

        let file_size = metadata.len();
        println!("Размер файла: {:?}", file_size);

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&self.program_input.input_filename);

        let mut file = self.check_file_access(file);

        let mut buffer = Vec::new();
        if let Ok(_) = file.read_to_end(&mut buffer) {
            println!("Содержимое файла (байты): {:?}", buffer);
        }
    }

    fn check_file_access<T, Err: std::fmt::Debug>(&mut self, result: Result<T, Err>) -> T {
        match result {
            Ok(t) => t,
            Err(err) => {
                eprintln!("File {} cannot be opened: {:?}", self.program_input.input_filename, err);
                panic!();
            }
        }
    }

}
