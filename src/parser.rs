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
                    'g' | 'G' => 9,
                    'm' | 'M' => 6,
                    'k' | 'K' => 3,
                    'b' | 'B' => 0,
                    _ => panic!()
                };
            },
            _ => return Err(())
        }
    }

    value *= 10usize.pow(if exp >= 0 { exp as u32 } else { 0 });

    return Ok(value);
}

pub struct ProgramInput {
    pub input_filename: String,
    pub fragment_size: usize,
}

pub enum ParseResult {
    Success(ProgramInput),
    ThereIsNoInputFilename,
    MemoryValueCannotBeParsed(String)
}

impl ProgramInput {
    pub fn parse(arguments: &Vec<String>) -> ParseResult {
        let _exe_name = &arguments[0];

        if arguments.len() <= 1 {
            return ParseResult::ThereIsNoInputFilename;
        }

        let mut input_filename: Option<String> = None;
        let mut fragment_size = 4 * 1024 * 1024 * 1024usize;

        let mut i = 1usize;
        while i < arguments.len() {
            let string = &arguments[i];

            match string.as_str() {
                "-s" | "--size" => {
                    i += 1;
                    fragment_size = match parse_memory_value(string) {
                        Ok(v) => v,
                        Err(_) => return ParseResult::MemoryValueCannotBeParsed(string.clone()),
                    }
                },
                _ => {
                    if input_filename == None {
                        input_filename = Option::Some(arguments[i].clone());
                    }
                }
            };
            i += 1;
        }

        return ParseResult::Success(ProgramInput{input_filename: arguments[1].clone(), fragment_size});
    }
}
