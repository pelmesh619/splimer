use std::cmp::min;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::parser::ProgramInput;

const MAX_BUFFER_SIZE: usize = 1024 * 1024usize;

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
            println!("File {} is already less than {} kB, no work is done!", 
                self.program_input.input_filename, 
                self.program_input.fragment_size / 1024
            );
            return;
        }
        let part_number = self.program_input.part_number;
        if part_number.is_some() && 
            ((file_size as f32) / self.program_input.fragment_size as f32).ceil() < part_number.unwrap() as f32 {
            println!("Error: Cannot generate {}{} part because there will be {} part{} in total", 
                part_number.unwrap(),
                match part_number.unwrap() {
                    1 => "st",
                    2 => "nd",
                    3 => "rd",
                    _ => "th"
                },
                ((file_size as f32) / self.program_input.fragment_size as f32).ceil() as usize,
                if ((file_size as f32) / self.program_input.fragment_size as f32).ceil() == 1f32 { "" } else { "s" }
            );
            return;
        }

        if let Some(parts) = self.program_input.parts {
            self.program_input.fragment_size = (file_size + parts - 1) / parts;
        }
        
        let start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let mut buffer = vec![0; min(MAX_BUFFER_SIZE, self.program_input.fragment_size)];

        let mut fragment_number = 1;
        let mut bytes_written = 0;
        let mut total_bytes_written = 0;

        self.open_file_for_write(&self.make_output_filename(fragment_number, &self.program_input.input_filename));
                
        while let Ok(size) = file.read(&mut buffer) {
            if size == 0 {
                break;
            }

            let how_many = min(size, self.program_input.fragment_size - bytes_written);
            self.write_bytes(buffer[..how_many].as_ref());

            bytes_written += how_many;
            if bytes_written == self.program_input.fragment_size {
                self.flush();
                total_bytes_written += bytes_written;
                println!("File {} is written, total written - {:0fill$} kB  /  {} kB", 
                    self.make_output_filename(fragment_number, &self.program_input.input_filename),
                    total_bytes_written / 1024,
                    file_size / 1024,
                    fill = (file_size / 1024).to_string().len()
                );
                if file_size == bytes_written * fragment_number as usize {
                    return;
                }
                fragment_number += 1;
                self.open_file_for_write(&self.make_output_filename(fragment_number, &self.program_input.input_filename));
                bytes_written = size - how_many;

                if how_many == size {
                    continue;
                }
                self.write_bytes(buffer[how_many..].as_ref());
            }
        }
        self.flush();
        total_bytes_written += bytes_written;
        println!("File {} is written, total written - {:0fill$} kB  /  {} kB", 
            self.make_output_filename(fragment_number, &self.program_input.input_filename),
            total_bytes_written / 1024,
            file_size / 1024,
            fill = (file_size / 1024).to_string().len()
        );

        println!(
            "The job is done! Total passed {:?} s", 
            (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() - start) as f64 / 1000f64
        );

    }

    pub fn merge(&mut self) {
        self.current_file_to_write = Some(
            Self::check_file_access(
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(Self::make_filename_with_suffix(&"_[merged]".to_string(), &self.program_input.input_filename))
            )
        );

        let start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let mut buffer = vec![0; MAX_BUFFER_SIZE];

        let mut fragment_number = 1;
        let mut bytes_written = 0usize;

        while let Ok(_) = fs::metadata(self.make_output_filename(fragment_number, &self.program_input.input_filename)) {
            let mut file = Self::check_file_access(
                OpenOptions::new()
                    .read(true)
                    .open(self.make_output_filename(fragment_number, &self.program_input.input_filename)
                )
            );

            while let Ok(size) = file.read(&mut buffer) {
                if size == 0 {
                    break;
                }
    
                self.write_bytes(buffer[..size].as_ref());
                bytes_written += size;
            }

            self.flush();
            println!("File {} is read, total kilobytes written - {}", 
                self.make_output_filename(fragment_number, &self.program_input.input_filename),
                bytes_written / 1024
            );

            fragment_number += 1;


        }
        println!("File {} was merged", &self.program_input.input_filename);

        println!(
            "The job is done! Total passed {:?} s", 
            (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() - start) as f64 / 1000f64
        );

    }

    fn write_bytes(&mut self, buffer: &[u8]) {
        if let Some(f) = &mut self.current_file_to_write {
            Self::check_file_access(f.write(buffer));
        }
    }

    fn open_file_for_write(&mut self, filename: &String) {
        self.current_file_to_write = Some(
            Self::check_file_access(
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(filename)
            )
        );
    }

    fn flush(&mut self) {
        if let Some(f) = &mut self.current_file_to_write {
            Self::check_file_access(f.flush());
        }
    }

    fn make_output_filename(&self, fragment_number: i32, pattern: &String) -> String {
        let filename = Path::new(pattern).file_stem().unwrap().to_str().unwrap();

        let filename = filename.to_string() + 
            "_[" + &fragment_number.to_string().to_owned() + "].splm";

        if let Some(dir) = &self.program_input.output_directory {
            Path::new(&dir)
                .join(filename)
                .to_str().unwrap().to_string()
        } else {
            return Path::new(pattern).parent().unwrap()
                .join(Path::new(&filename))
                .to_str().unwrap().to_string()
        }        
    }

    fn make_filename_with_suffix(suffix: &String, pattern: &String) -> String {        
        return Path::new(pattern).file_stem().unwrap().to_str().unwrap().to_string() + 
            suffix + 
            "." + 
            Path::new(pattern).extension().unwrap().to_str().unwrap();
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
