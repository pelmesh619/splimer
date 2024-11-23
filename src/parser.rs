const DEFAULT_FRAGMENT_SIZE: usize = 1024 * 1024 * 1024usize;
const MINIMUM_FRAGMENT_SIZE: usize = 1024;
const DEFAULT_OUTPUT_DIRECTORY: &str = "splimer-output";

pub fn parse_memory_value(string: &String) -> Result<usize, ()> {
    let mut value = 0usize;
    let mut exp = -1;

    for i in string.chars().into_iter() {
        match i {
            '0'..='9' => {
                value *= 10;
                value += i.to_digit(10u32).unwrap_or(0u32) as usize;

                if exp != -1 { exp += 1; }
            },
            '.' => {
                if exp == -1 {
                    exp = 0;
                } else {
                    return Err(());
                }
            },
            'g' | 'G' | 'm' | 'M' | 'k' | 'K' | 'b' | 'B' => {
                if exp == -1 {
                    exp = 0;
                }
                exp += match i {
                    'g' | 'G' => 30,
                    'm' | 'M' => 20,
                    'k' | 'K' => 10,
                    'b' | 'B' => 0,
                    _ => panic!()
                };
            },
            _ => return Err(())
        }
    }

    value *= 2usize.pow(if exp >= 0 { exp as u32 } else { 0 });

    return Ok(value);
}

pub struct ProgramInput {
    pub to_split: bool,
    pub input_filename: String,
    pub fragment_size: usize,
    pub output_directory: String,
}

struct ProgramInputBuilder {
    pub to_split: bool,
    pub input_filename: Option<String>,
    pub fragment_size: usize,
    pub output_directory: String,
}

impl ProgramInputBuilder {
    fn new() -> ProgramInputBuilder {
        return Self{
            to_split: true,
            input_filename: None,
            fragment_size: DEFAULT_FRAGMENT_SIZE,
            output_directory: DEFAULT_OUTPUT_DIRECTORY.to_string()
        }
    }
}

pub enum ParseResult {
    Success(ProgramInput),
    ThereIsNoInputFilename,
    MemoryValueCannotBeParsed(String),
    FragmentSizeIsToSmall(usize),
    ThereIsNoValue(String),
    SuccessfulHandledArgument,
    SuccessfulHandledFlag,
}

impl ProgramInput {
    pub fn parse(arguments: &Vec<String>) -> ParseResult {
        let _exe_name = &arguments[0];

        if arguments.len() <= 1 {
            return ParseResult::ThereIsNoInputFilename;
        }

        let mut builder = ProgramInputBuilder::new();

        let mut i = 1usize;
        while i < arguments.len() {
            let string = &arguments[i];

            let key;
            let value;
            let is_next_argument_a_value;
            if let Some(equal_sign_index) = string.find('=') {
                value = string[equal_sign_index + 1..].to_string();
                key = string[..equal_sign_index].to_string();
                is_next_argument_a_value = false;
            } else {
                key = string.clone();
                value = if i + 1 < arguments.len() {
                    is_next_argument_a_value = true;
                    arguments[i + 1].clone()
                } else {
                    is_next_argument_a_value = false;
                    String::new()
                }
            }

            println!("{} {}", key, value);

            let result = Self::handle_argument(&key, &value, &mut builder);
            match result {
                ParseResult::SuccessfulHandledArgument => { if is_next_argument_a_value { i += 1; } },
                ParseResult::SuccessfulHandledFlag => { },
                ParseResult::Success(_) => panic!(),
                _ => return result
            }
            
            i += 1;
        }

        if builder.input_filename == None {
            return ParseResult::ThereIsNoInputFilename;
        }

        return ParseResult::Success(
            ProgramInput{
                to_split: builder.to_split,
                input_filename: builder.input_filename.unwrap(), 
                fragment_size: builder.fragment_size,
                output_directory: builder.output_directory.to_string()
            }
        );
    }

    fn handle_argument(key: &String, value: &String, builder: &mut ProgramInputBuilder) -> ParseResult {
        match key.as_str() {
            "-S" | "--fragment-size" => {
                if value.is_empty() {
                    return ParseResult::ThereIsNoValue(key.clone());
                }
                builder.fragment_size = match parse_memory_value(&value) {
                    Ok(v) => v,
                    Err(_) => return ParseResult::MemoryValueCannotBeParsed(value.clone()),
                };
                if builder.fragment_size < MINIMUM_FRAGMENT_SIZE {
                    return ParseResult::FragmentSizeIsToSmall(MINIMUM_FRAGMENT_SIZE);
                };
                return ParseResult::SuccessfulHandledArgument;
            },
            "-o" | "--output-directory" => {
                if value.is_empty() {
                    return ParseResult::ThereIsNoValue(key.clone());
                }
                builder.output_directory = value.clone();
                return ParseResult::SuccessfulHandledArgument;
            },
            "-m" | "--merge" => {
                builder.to_split = false;
                return ParseResult::SuccessfulHandledFlag;
            },
            "-s" | "--split" => {
                builder.to_split = true;
                return ParseResult::SuccessfulHandledFlag;
            }
            _ => {
                if builder.input_filename == None {
                    builder.input_filename = Some(key.clone());
                    return ParseResult::SuccessfulHandledFlag;
                }
                println!("Warning: unknown argument - {}", key);
                return ParseResult::SuccessfulHandledFlag;
            }
        };
    }
}
