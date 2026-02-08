use std::error::Error;
use std::fmt;

#[derive(Default)]
pub struct Options {
    pub ignore_case: bool,
    pub line_number: bool,
    pub invert_match: bool,
    pub line_regexp: bool,
    pub with_filename: Option<bool>,
    pub quiet: bool,
    pub count: bool,
    pub max_count: Option<usize>,
}

#[derive(Debug)]
pub enum ParseError {
    NotEnoughArguments,
    MissingMaxCountValue,
    InvalidMaxCountValue(String),
    UnknownArgument(String),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::NotEnoughArguments => write!(f, "not enough arguments"),
            ParseError::MissingMaxCountValue => write!(f, "missing value for max-count"),
            ParseError::InvalidMaxCountValue(value) => {
                write!(f, "invalid value for max-count: {value}")
            }
            ParseError::UnknownArgument(arg) => write!(f, "unknown argument: {arg}"),
        }
    }
}

impl Error for ParseError {}

pub struct Config {
    pub(crate) query: String,
    pub(crate) filenames: Vec<String>,
    pub(crate) options: Options,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, ParseError> {
        let (query, filenames, options) = parse_args(args)?;

        Ok(Config {
            query,
            filenames,
            options,
        })
    }
}

fn parse_args(args: &[String]) -> Result<(String, Vec<String>, Options), ParseError> {
    let mut options = Options::default();
    let mut positionals = Vec::new();
    let mut index = 1;

    while index < args.len() {
        let arg = &args[index];
        index += 1;

        if arg.starts_with("--") {
            parse_long_option(arg, &mut options, args, &mut index)?;
        } else if arg.starts_with('-') {
            parse_short_option(arg, &mut options, args, &mut index)?;
        } else {
            positionals.push(arg.clone());
        }
    }

    if positionals.len() < 2 {
        return Err(ParseError::NotEnoughArguments);
    }

    let query = positionals[0].clone();
    let filenames = positionals[1..].to_vec();
    Ok((query, filenames, options))
}

fn parse_long_option(
    arg: &str,
    options: &mut Options,
    args: &[String],
    index: &mut usize,
) -> Result<(), ParseError> {
    match arg {
        "--ignore-case" => options.ignore_case = true,
        "--line-number" => options.line_number = true,
        "--invert-match" => options.invert_match = true,
        "--line-regexp" => options.line_regexp = true,
        "--with-filename" => options.with_filename = Some(true),
        "--no-filename" => options.with_filename = Some(false),
        "--quiet" | "--silent" => options.quiet = true,
        "--count" => options.count = true,
        "--max-count" => {
            let count_str = args.get(*index).ok_or(ParseError::MissingMaxCountValue)?;
            *index += 1;
            let count = count_str
                .parse::<usize>()
                .map_err(|_| ParseError::InvalidMaxCountValue(count_str.clone()))?;
            options.max_count = Some(count);
        }
        _ => return Err(ParseError::UnknownArgument(arg.to_string())),
    }
    Ok(())
}

fn parse_short_option(
    arg: &str,
    options: &mut Options,
    args: &[String],
    index: &mut usize,
) -> Result<(), ParseError> {
    let chars: Vec<char> = arg.chars().collect();
    let mut char_index = 1;
    while char_index < chars.len() {
        let c = chars[char_index];
        char_index += 1;

        match c {
            'i' | 'y' => options.ignore_case = true,
            'n' => options.line_number = true,
            'v' => options.invert_match = true,
            'x' => options.line_regexp = true,
            'H' => options.with_filename = Some(true),
            'h' => options.with_filename = Some(false),
            'q' => options.quiet = true,
            'c' => options.count = true,
            'm' => {
                let count_str = if char_index < chars.len() {
                    let s: String = chars[char_index..].iter().collect();
                    char_index = chars.len();
                    s
                } else {
                    let s = args.get(*index).ok_or(ParseError::MissingMaxCountValue)?;
                    *index += 1;
                    s.clone()
                };

                let count = count_str
                    .parse::<usize>()
                    .map_err(|_| ParseError::InvalidMaxCountValue(count_str.clone()))?;

                options.max_count = Some(count);
            }
            _ => return Err(ParseError::UnknownArgument(format!("-{}", c))),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config_with_options_before_positionals() {
        let args = vec![
            "minigrep".to_string(),
            "-i".to_string(),
            "duct".to_string(),
            "poem.txt".to_string(),
        ];

        let config = Config::new(&args).expect("failed to parse args");
        assert_eq!("duct", config.query);
        assert_eq!(vec!["poem.txt".to_string()], config.filenames);
        assert!(config.options.ignore_case);
    }

    #[test]
    fn parse_config_with_options_between_positionals() {
        let args = vec![
            "minigrep".to_string(),
            "duct".to_string(),
            "-i".to_string(),
            "poem.txt".to_string(),
        ];

        let config = Config::new(&args).expect("failed to parse args");
        assert_eq!("duct", config.query);
        assert_eq!(vec!["poem.txt".to_string()], config.filenames);
        assert!(config.options.ignore_case);
    }

    #[test]
    fn parse_config_with_options_after_positionals() {
        let args = vec![
            "minigrep".to_string(),
            "duct".to_string(),
            "poem.txt".to_string(),
            "-i".to_string(),
        ];

        let config = Config::new(&args).expect("failed to parse args");
        assert_eq!("duct", config.query);
        assert_eq!(vec!["poem.txt".to_string()], config.filenames);
        assert!(config.options.ignore_case);
    }

    #[test]
    fn parse_config_rejects_double_dash() {
        let args = vec![
            "minigrep".to_string(),
            "--".to_string(),
            "-i".to_string(),
            "poem.txt".to_string(),
        ];

        match Config::new(&args) {
            Err(ParseError::UnknownArgument(arg)) => assert_eq!("--", arg),
            Err(other) => panic!("unexpected parse error: {other}"),
            Ok(_) => panic!("expected parse to fail"),
        }
    }

    #[test]
    fn parse_config_with_multiple_files() {
        let args = vec![
            "minigrep".to_string(),
            "duct".to_string(),
            "poem.txt".to_string(),
            "more.txt".to_string(),
        ];

        let config = Config::new(&args).expect("failed to parse args");
        assert_eq!("duct", config.query);
        assert_eq!(
            vec!["poem.txt".to_string(), "more.txt".to_string()],
            config.filenames
        );
    }
}
