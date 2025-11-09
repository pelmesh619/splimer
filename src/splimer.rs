use std::cmp::min;
use std::fs;
use std::fs::{File, OpenOptions};
use std::io::{BufReader, Read, Seek, SeekFrom, Write};
use std::io;
use regex::Regex;
use std::path::{Component, Path, PathBuf};
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
    fn make_dir_file(&mut self) -> io::Result<()> {
        let output_file = &self.make_output_dir_filename(&self.program_input.input_filename);
        let root_path = Path::new(&self.program_input.input_filename).canonicalize()?;
        
        let files = Self::collect_files(&root_path)?;
        
        let mut offset = 0;
        self.records = Vec::new();
        
        for (path, size) in files {            
            let relative_path = path.strip_prefix(&root_path)
                .unwrap_or(&path);

            let relative_path = to_unix_path(relative_path);
            
            self.records.push(FileRecord {
                path: relative_path,
                size,
                offset,
                fragment_index: 0,
            });
            
            offset += size;
        }
        if self.records.len() == 0 {
            return Ok(())
        }
        
        let file_size = {
            let r = self.records.last().unwrap();
            r.offset + r.size
        };
        if let Some(parts) = self.program_input.parts {
            self.program_input.fragment_size = (file_size + parts - 1) / parts;
        }
        for f in &mut self.records {
            f.fragment_index = f.offset / self.program_input.fragment_size
        }

        let file = File::create(output_file)?;
        serde_json::to_writer_pretty(file, &self.records)?;
        
        Ok(())
    }

    fn collect_files(root: &Path) -> io::Result<Vec<(PathBuf, usize)>> {
        let mut files = Vec::new();
        Self::_collect_files(root, &mut files)?;
        Ok(files)
    }
    fn _collect_files(current_dir: &Path, files: &mut Vec<(PathBuf, usize)>) -> io::Result<()> {
        for entry in fs::read_dir(current_dir)? {
            let entry = entry?;
            let path = entry.path();
            let metadata = fs::metadata(&path)?;
    
            if metadata.is_file() {
                files.push((path, metadata.len() as usize));
            } else if metadata.is_dir() {
                Self::_collect_files(&path, files)?;
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
        let dir_filename = &self.make_output_dir_filename(&self.program_input.input_filename);
        if metadata.is_dir() {
            parent_directory = Path::new(full_path);
            self.make_dir_file().expect(format!("There is some error in creating directory file {}", dir_filename).as_str());
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

        let binding = self.make_output_filename(fragment_number, &self.program_input.input_filename, true);
        let fragment_filepath= Path::new(&binding);
        let fragment_filepath = Path::new(self.program_input.output_directory.clone().unwrap_or(String::new()).as_str()).join(fragment_filepath.file_name().unwrap());
        self.open_file_for_write(&fragment_filepath.to_str().unwrap().to_string());

        
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
                    self.log_fragment_written(fragment_number, total_bytes_written, file_size);
                    fragment_number += 1;

                    let binding = self.make_output_filename(fragment_number, &self.program_input.input_filename, true);
                    let fragment_filepath= Path::new(&binding);
                    let fragment_filepath = Path::new(self.program_input.output_directory.clone().unwrap_or(String::new()).as_str()).join(fragment_filepath.file_name().unwrap());
                    self.open_file_for_write(&fragment_filepath.to_str().unwrap().to_string());
            
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

        self.log_fragment_written(fragment_number, total_bytes_written, file_size);

        if !self.program_input.is_quiet {
            println!(
                "The job is done! Total passed {:?} s", 
                (SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() - start) as f64 / 1000f64
            );
        }
    }

    fn log_fragment_written(&self, fragment_number: usize, total_bytes_written: usize, file_size: usize) {
        if !self.program_input.is_quiet {
            println!("File {} is written, total written - {:0fill$} kB  /  {} kB", 
                self.make_output_filename(fragment_number, &self.program_input.input_filename, true),
                total_bytes_written / 1024,
                file_size / 1024,
                fill = (file_size / 1024).to_string().len()
            );
        }
    }

    pub fn merge(&mut self) {
        self.strip_input_suffix();
    
        let input_filename = &self.program_input.input_filename.clone();
        let output_directory = self.make_output_directory_path();
    
        let first_fragment_path = self.get_first_fragment_path(input_filename);
        let file_path = self.get_sanitized_file_path(&first_fragment_path);
    
        let dir_filename = self.make_dir_filename(&file_path);
        let is_single_file = match fs::metadata(&dir_filename) {
            Ok(meta) if meta.is_file() => {
                let dir_file = Self::check_file_access(OpenOptions::new().read(true).open(&dir_filename));
                self.read_metadata(&dir_file).expect("Failed to read metadata");
                if self.records.is_empty() {
                    eprintln!("{} contains no files, no work is done", input_filename);
                    return;
                }
                false
            }
            _ => {
                self.records = vec![FileRecord {
                    path: input_filename.clone(),
                    size: 0,
                    offset: 0,
                    fragment_index: 0,
                }];
                true
            }
        };
    
        let start_time = Self::current_time_millis();
        let mut buffer = vec![0; MAX_BUFFER_SIZE];
    
        let mut file_to_write_index = 0;
        let mut fragment_number = 1;
        let mut bytes_written = 0usize;
    
        let mut buffer_size = 0;
        let mut buffer_offset = 0;
    
        let mut file_to_read = Self::check_file_access(
            OpenOptions::new()
                .read(true)
                .open(self.make_output_filename(fragment_number, input_filename, is_single_file)),
        );
    
        while file_to_write_index < self.records.len() {
            let file_record = &self.records[file_to_write_index];
            self.ensure_output_directory(&output_directory, &file_record.path);
    
            self.current_file_to_write = Some(Self::check_file_access(
                OpenOptions::new()
                    .write(true)
                    .truncate(true)
                    .create(true)
                    .open(output_directory.join(&file_record.path)),
            ));
    
            let (mut bytes_read, file_size, file_offset, file_fragment_index) =
                (0, file_record.size, file_record.offset, file_record.fragment_index);
            let file_path = file_record.path.clone();
    
            if buffer_offset < buffer_size {
                // some bytes have left in the buffer
                let how_many = min(buffer_size - buffer_offset, file_size);
                self.write_bytes(&buffer[buffer_offset..buffer_offset + how_many]);
                buffer_offset += how_many;
                bytes_read += how_many;
                bytes_written += how_many;
    
                if buffer_size > buffer_offset {
                    self.flush();
                    self.log_file_written(&file_path, bytes_written);
                    file_to_write_index += 1;
                    continue;
                }
                buffer_offset = 0;
            }
    
            // read fragments loop
            while bytes_read < file_size || is_single_file {
                if file_fragment_index + 1 > fragment_number {
                    fragment_number = file_fragment_index + 1;
                    file_to_read = Self::check_file_access(
                        OpenOptions::new()
                            .read(true)
                            .open(self.make_output_filename(fragment_number, input_filename, is_single_file)),
                    );
                    file_to_read
                        .seek(SeekFrom::Start(file_offset as u64))
                        .expect("Failed to seek in fragment");
                }
    
                while let Ok(size) = file_to_read.read(&mut buffer) {
                    if size == 0 {
                        self.log_fragment_read(fragment_number, input_filename, bytes_written);
                        fragment_number += 1;
                        buffer_offset = 0;
    
                        if file_to_write_index + 1 == self.records.len() && file_size == bytes_read {
                            break;
                        }
    
                        match OpenOptions::new()
                            .read(true)
                            .open(self.make_output_filename(fragment_number, input_filename, is_single_file))
                        {
                            Ok(next_file) => file_to_read = next_file,
                            Err(_) if is_single_file => break,
                            Err(_) => panic!("Missing fragment file."),
                        }
    
                        break;
                    }
    
                    buffer_size = size;
                    let how_many = if is_single_file {
                        buffer_size
                    } else {
                        min(buffer_size, file_size - bytes_read)
                    };
    
                    self.write_bytes(&buffer[..how_many]);
                    buffer_offset = how_many;
                    bytes_read += how_many;
                    bytes_written += how_many;
    
                    if buffer_size > how_many {
                        self.flush();
                        break;
                    }
                }
    
                self.flush();
            }
    
            self.log_file_written(&file_path, bytes_written);
            file_to_write_index += 1;
        }
    
        if !self.program_input.is_quiet {
            println!(
                "{} {} was merged",
                if is_single_file { "File" } else { "Directory" },
                input_filename
            );
        
            println!(
                "The job is done! Total passed {:.3} s",
                (Self::current_time_millis() - start_time) as f64 / 1000.0
            );
        }
    }
    
    fn strip_input_suffix(&mut self) {
        let input = &mut self.program_input.input_filename;
        if input.ends_with(".dir.splm") {
            *input = input.strip_suffix(".dir.splm").unwrap().to_string();
        }
    }
    
    fn make_output_directory_path(&self) -> PathBuf {
        let input_filename = Path::new(&self.program_input.input_filename);
        let output_dir = self
            .program_input
            .output_directory
            .clone()
            .unwrap_or_default();
    
        Path::new(&output_dir).join(input_filename.file_name().unwrap())
    }
    
    fn get_first_fragment_path(&self, input_filename: &str) -> PathBuf {
        let path = self.make_output_filename(1, &input_filename.to_string(), false);
        fs::canonicalize(&path)
            .unwrap_or_else(|_| panic!("Failed to canonicalize path: {}", path))
    }
    
    fn get_sanitized_file_path(&self, path: &Path) -> PathBuf {
        let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or_else(|| {
            path.as_os_str()
                .to_str()
                .expect("Invalid UTF-8 in file path")
        });
        let joined = path.parent().unwrap().join(stem);
        Path::new(
            joined
                .to_str()
                .unwrap()
                .strip_prefix(r"\\?\")
                .unwrap_or(joined.to_str().unwrap()),
        )
        .to_path_buf()
    }
    
    fn make_dir_filename(&self, path: &Path) -> String {
        let re = Regex::new(r"_\[\d+\]$").unwrap();
        let mut base = re.replace(path.to_str().unwrap(), "").to_string();
        base.push_str(".dir.splm");
        base
    }
    
    fn ensure_output_directory(&self, base: &Path, relative: &str) {
        if let Some(parent) = Path::new(relative).parent() {
            let full = base.join(parent);
            if !full.exists() {
                fs::create_dir_all(&full).expect("Cannot create output directory");
            }
        }
    }
    
    fn log_file_written(&self, path: &str, bytes: usize) {
        if !self.program_input.is_quiet {
            println!("File {} is written, total written - {} kB", path, bytes / 1024);
        }
    }
    
    fn log_fragment_read(&self, number: usize, name: &str, bytes: usize) {
        if !self.program_input.is_quiet {
            println!(
                "File {} is read, total read - {} kB",
                self.make_output_filename(number, &name.to_string(), false),
                bytes / 1024
            );
        }
    }
    
    fn current_time_millis() -> u128 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
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

    fn make_output_filename(&self, fragment_number: usize, pattern: &String, remove_ext: bool) -> String {
        let filename = Path::new(pattern);
        let filename = if remove_ext {
            if let Some(f) = filename.file_stem() {
                f
            } else {
                filename.parent().unwrap().file_stem().unwrap()
            }
        } else {
            filename.file_name().unwrap()
        }.to_str().unwrap();

        let filename = filename.to_string() + 
            "_[" + &fragment_number.to_string().to_owned() + "].splm";

        Path::new(pattern).parent().unwrap()
            .join(Path::new(&filename))
            .to_str().unwrap().to_string()
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

        Path::new(&self.program_input.output_directory.clone().unwrap_or(String::new()))
            .join(filename)
            .to_str().unwrap().to_string()
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

fn to_unix_path(path: &Path) -> String {
    let mut components = path.components();
    let mut unix_path = String::new();

    while let Some(component) = components.next() {
        match component {
            Component::Prefix(prefix) => {
                if let Some(disk) = prefix.as_os_str().to_str().and_then(|s| s.strip_suffix(':')) {
                    unix_path.push_str(&disk.to_lowercase());
                    unix_path.push_str("/");
                }
            }
            Component::RootDir => {
                unix_path.push('/');
            }
            Component::CurDir => {
                unix_path.push_str("./");
            }
            Component::ParentDir => {
                unix_path.push_str("../");
            }
            Component::Normal(name) => {
                if let Some(name_str) = name.to_str() {
                    unix_path.push_str(name_str);
                }
                if components.as_path() != Path::new("") {
                    unix_path.push('/');
                }
            }
        }
    }

    unix_path
}
