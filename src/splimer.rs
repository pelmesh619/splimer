use std::cmp::min;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write, Seek, SeekFrom};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};
use serde_json;

use crate::parser::ProgramInput;

const MAX_BUFFER_SIZE: usize = 1024 * 1024usize; // in bytes

#[derive(Debug, Serialize, Deserialize)]
struct FileRecord {
    path: String,
    size: usize,
    offset: usize,
    fragment_index: usize,
}

pub struct Splimer {
    pub program_input: ProgramInput,
    current_file_to_write: Option<File>,
    records: Vec<FileRecord>
}

impl Splimer {
    pub fn new(program_input: ProgramInput) -> Splimer {
        return Splimer{
            program_input, 
            current_file_to_write: None,
            records: Vec::new()
        };
    }
    pub fn make_dir_file(&mut self) {
        let _ = self.scan_directory(
            &self.make_output_dir_filename(&self.program_input.input_filename)
        );
    }
    
    fn scan_directory(&mut self, output_file: &str) -> io::Result<()> {
        let root_path = Path::new(&self.program_input.input_filename).canonicalize()?;
        
        let files = Self::collect_files_recursively(&root_path)?;
        
        let mut offset = 0;
        self.records = Vec::new();
        
        for (path, size) in files {
            let fragment_index = offset / self.program_input.fragment_size;
            
            let relative_path = path.strip_prefix(&root_path)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            
            self.records.push(FileRecord {
                path: relative_path,
                size,
                offset,
                fragment_index,
            });
            
            offset += size;
        }

        let file = File::create(output_file)?;
        serde_json::to_writer_pretty(file, &self.records)?;
        
        Ok(())
    }

    fn collect_files_recursively(root: &Path) -> io::Result<Vec<(PathBuf, usize)>> {
        let mut files = Vec::new();
        Self::collect_files(root, &mut files)?;
        Ok(files)
    }
    fn collect_files(current_dir: &Path, files: &mut Vec<(PathBuf, usize)>) -> io::Result<()> {
        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = fs::metadata(&path)?;
    
            if metadata.is_file() {
                files.push((path, metadata.len() as usize));
            } else if metadata.is_dir() {
                Self::collect_files(&path, files)?;
            }
        }
        Ok(())
    }



    pub fn split(&mut self) {    
        let full_path = fs::canonicalize(&self.program_input.input_filename).expect("Failed to canonicalize path");
    
        let full_path = if let Some(full_path_str) = full_path.to_str() {
            full_path_str.strip_prefix(r"\\?\").unwrap_or(full_path_str)
        } else {
            full_path.to_str().unwrap()
        };

        let metadata = Self::check_file_access(fs::metadata(full_path));
        let parent_directory;
        if metadata.is_dir() {
            parent_directory = Path::new(full_path);
            self.make_dir_file();
            if self.records.is_empty() {
                println!("The directory {} is empty to split, no work is done", 
                    self.program_input.input_filename
                );
                return;
            }
        } else if metadata.is_file() {
            parent_directory = Path::new(full_path).parent().unwrap();
            self.records = vec!(
                FileRecord { 
                    path: self.program_input.input_filename.clone(), 
                    size: metadata.len() as usize, 
                    offset: 0usize, 
                    fragment_index: 0usize
                }
            );
        } else {
            println!("Path {} is neither file nor directory, no work is done", 
                self.program_input.input_filename, 
            );
            return;
        }
        let file_size = {
            let r = self.records.last().unwrap();
            r.offset + r.size
        };

        if let Some(parts) = self.program_input.parts {
            self.program_input.fragment_size = (file_size + parts - 1) / parts;
        }

        if file_size < self.program_input.fragment_size {
            println!("{} {} is already less than {} kB, no work is done!", 
                if metadata.is_file() { "File" } else { "Directory" },
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
        
        let start = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let mut buffer = vec![0; min(MAX_BUFFER_SIZE, self.program_input.fragment_size)];

        let mut fragment_number = self.program_input.part_number.unwrap_or(1);
        let mut bytes_written = 0;
        let mut total_bytes_written = 0;
        let mut file_to_read_index = 0;

        let file_size = part_number.and(Some(self.program_input.fragment_size)).or(Some(file_size)).unwrap();

        self.open_file_for_write(&self.make_output_filename(fragment_number, &self.program_input.input_filename));

        
        while file_to_read_index < self.records.len() {
            let file_record = &self.records[file_to_read_index];
            let next_file_record = if file_to_read_index + 1 >= self.records.len() { None } else { Some(&self.records[file_to_read_index + 1]) };
            if part_number.is_some() { 
                if next_file_record.is_some() && next_file_record.unwrap().fragment_index + 1 < part_number.unwrap() {
                    file_to_read_index += 1;
                    continue;
                }
                if file_record.fragment_index + 1 > fragment_number {
                    break;
                }
            }

            let file = OpenOptions::new()
                .read(true)
                .open(Path::new(parent_directory).join(file_record.path.as_str()));
            let mut file = Self::check_file_access(file);

            if self.program_input.part_number.is_some() && file_record.fragment_index + 1 < part_number.unwrap() {
                Self::check_file_access(
                    file.seek(SeekFrom::Start(((file_record.fragment_index + 1) * self.program_input.fragment_size - file_record.offset) as u64))
                );
            }

            while let Ok(size) = file.read(&mut buffer) {
                if size == 0 {
                    break;
                }

                let how_many = min(size, self.program_input.fragment_size - bytes_written);
                self.write_bytes(buffer[..how_many].as_ref());

                bytes_written += how_many;
                if bytes_written == self.program_input.fragment_size {
                    self.flush();
                    if file_size == total_bytes_written + bytes_written {
                        break; // all fragments are written
                    }
                    total_bytes_written += bytes_written;
                    println!("File {} is written, total written - {:0fill$} kB  /  {} kB", 
                        self.make_output_filename(fragment_number, &self.program_input.input_filename),
                        total_bytes_written / 1024,
                        file_size / 1024,
                        fill = (file_size / 1024).to_string().len()
                    );
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
            file_to_read_index += 1;
        }

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

    fn make_output_filename(&self, fragment_number: usize, pattern: &String) -> String {
        let filename = Path::new(pattern);
        let filename = if let Some(f) = filename.file_stem() {
            f
        } else {
            filename.parent().unwrap().file_stem().unwrap()
        }.to_str().unwrap();

        let filename = filename.to_string() + 
            "_[" + &fragment_number.to_string().to_owned() + "].splm";

        if let Some(dir) = &self.program_input.output_directory {
            Path::new(&dir)
                .join(filename)
                .to_str().unwrap().to_string()
        } else {
            Path::new(pattern).parent().unwrap()
                .join(Path::new(&filename))
                .to_str().unwrap().to_string()
        }        
    }
    fn make_output_dir_filename(&self, pattern: &String) -> String {
        let full_path = fs::canonicalize(pattern).unwrap();
        let filename = Path::new(full_path.as_path());
        let filename = if let Some(f) = filename.file_stem() {
            f
        } else {
            filename.parent().unwrap().file_stem().unwrap()
        }.to_str().unwrap();

        let filename = filename.to_string() + ".dir.splm";

        if let Some(dir) = &self.program_input.output_directory {
            Path::new(&dir)
                .join(filename)
                .to_str().unwrap().to_string()
        } else {
            Path::new(pattern).parent().unwrap()
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

    fn read_metadata(&mut self, file: &File) -> io::Result<()> {
        let reader = BufReader::new(file);
        self.records = serde_json::from_reader(reader).expect("Directory file is corrupted and cannot be read!");

        Ok(())
    }

}
