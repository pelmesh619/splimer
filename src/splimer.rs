use std::cmp::min;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};

use crate::parser::ProgramInput;

const MAX_BUFFER_SIZE: usize = 1024usize;

pub struct Splimer {
    pub program_input: ProgramInput,
    current_file_to_write: Option<File>,
}

impl Splimer {
    pub fn new(program_input: ProgramInput) -> Splimer {
        return Splimer{
            program_input, 
            current_file_to_write: None
        };
    }

    pub fn split(&mut self) {
        let file = OpenOptions::new()
            .read(true)
            .open(&self.program_input.input_filename);
        let mut file = Self::check_file_access(file);

        let metadata = Self::check_file_access(file.metadata());

        let file_size = metadata.len() as usize;
        
        if file_size < self.program_input.fragment_size {
            println!("File {} is already less than {}, no work is done!", 
                self.program_input.input_filename, 
                self.program_input.fragment_size
            );
            return;
        }


        let mut buffer = vec![0; min(MAX_BUFFER_SIZE, self.program_input.fragment_size)];

        let mut fragment_number = 1;
        let mut bytes_written = 0;

        let result = OpenOptions::new()
            .write(true)
            .create(true)
            .open(Self::make_output_filename(fragment_number, &self.program_input.input_filename));
        
        println!("Содержимое файла {}", Self::make_output_filename(fragment_number, &self.program_input.input_filename));
        self.current_file_to_write = Some(Self::check_file_access(result));

        while let Ok(size) = file.read(&mut buffer) {
            if size == 0 {
                break;
            }

            let how_many = min(size, self.program_input.fragment_size - bytes_written);
            self.write_bytes(buffer[..how_many].as_ref());

            bytes_written += how_many;
            if bytes_written == self.program_input.fragment_size {
                self.current_file_to_write.as_ref().unwrap().flush();
                fragment_number += 1;
                self.current_file_to_write = Some(
                    Self::check_file_access(
                        OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .create(true)
                            .open(Self::make_output_filename(fragment_number, &self.program_input.input_filename))
                    )
                );
                bytes_written = size - how_many;

                if how_many == size {
                    continue;
                }
                self.write_bytes(buffer[how_many..].as_ref());
            }
        }

    }

    pub fn merge (&mut self) {
        self.current_file_to_write = Some(
            Self::check_file_access(
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(Self::make_filename_with_suffix(&"_[merged]".to_string(), &self.program_input.input_filename))
            )
        );

        let mut buffer = vec![0; self.program_input.fragment_size];

        let mut fragment_number = 1;
        let mut bytes_written = 0usize;

        while let Ok(_) = fs::metadata(Self::make_output_filename(fragment_number, &self.program_input.input_filename)) {
            let mut file = Self::check_file_access(
                OpenOptions::new()
                    .read(true)
                    .open(Self::make_output_filename(fragment_number, &self.program_input.input_filename)
                )
            );

            while let Ok(size) = file.read(&mut buffer) {
                if size == 0 {
                    break;
                }
    
                self.write_bytes(buffer[..size].as_ref());
                bytes_written += size;
            }

            println!("File {} is read, total kilobytes written - {}", 
                Self::make_output_filename(fragment_number, &self.program_input.input_filename),
                bytes_written / 1024
            );

            fragment_number += 1;

        }

    }

    fn write_bytes(&mut self, buffer: &[u8]) {
        if let Some(f) = &mut self.current_file_to_write {
            Self::check_file_access(f.write(buffer));
        }
    }

    fn make_output_filename(fragment_number: i32, pattern: &String) -> String {
        (
            if let Some(ind) = pattern.rfind('.') {
                pattern[..ind].to_string()
            } else {
                pattern.clone()
            }
        ) + &fragment_number.to_string().to_owned() + ".splm"
    }

    fn make_filename_with_suffix(suffix: &String, pattern: &String) -> String {
        if let Some(ind) = pattern.rfind('.') {
            pattern[..ind].to_string() + suffix + &pattern[ind..].to_string()
        } else {
            pattern.clone() + suffix
        }
    }

    fn check_file_access<T, Error: std::fmt::Debug>(result: Result<T, Error>) -> T {
        match result {
            Ok(t) => t,
            Err(err) => {
                eprintln!("File cannot be opened: {:?}", err);
                panic!();
            }
        }
    }

}
