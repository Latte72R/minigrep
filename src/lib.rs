use std::error::Error;
use std::fmt;
use std::fs::File;
use std::io::prelude::*;

#[derive(Default)]
pub struct Options {
    pub ignore_case: bool,
    pub line_number: bool,
    pub invert_match: bool,
    pub line_regexp: bool,
    pub with_filename: bool,
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
    query: String,
    filename: String,
    options: Options,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, ParseError> {
        let (query, filename, option_args) = parse_positional(args)?;
        let options = parse_options(option_args)?;

        Ok(Config {
            query,
            filename,
            options,
        })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MatchLine<'a> {
    pub line_no: usize,
    pub text: &'a str,
}

fn parse_positional<'a>(args: &'a [String]) -> Result<(String, String, &'a [String]), ParseError> {
    if args.len() < 3 {
        return Err(ParseError::NotEnoughArguments);
    }

    Ok((args[1].clone(), args[2].clone(), &args[3..]))
}

fn parse_options(args: &[String]) -> Result<Options, ParseError> {
    let mut options = Options::default();
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        index += 1;
        match arg.as_str() {
            "-i" | "-y" | "--ignore-case" => options.ignore_case = true,
            "-n" | "--line-number" => options.line_number = true,
            "-v" | "--invert-match" => options.invert_match = true,
            "-x" | "--line-regexp" => options.line_regexp = true,
            "-H" | "--with-filename" => options.with_filename = true,
            "-h" | "--no-filename" => options.with_filename = false,
            "-q" | "--quiet" | "--silent" => options.quiet = true,
            "-c" | "--count" => options.count = true,
            "-m" | "--max-count" => {
                let count_str = args.get(index).ok_or(ParseError::MissingMaxCountValue)?;
                index += 1;
                let count = count_str
                    .parse::<usize>()
                    .map_err(|_| ParseError::InvalidMaxCountValue(count_str.clone()))?;
                options.max_count = Some(count);
            }
            _ => return Err(ParseError::UnknownArgument(arg.clone())),
        }
    }

    Ok(options)
}

pub fn run(config: Config) -> Result<bool, Box<dyn Error>> {
    let mut f = File::open(config.filename.clone())?;

    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("something went wrong reading the file");

    let matches = search(&contents, &config);

    if config.options.count {
        let count = matches.len();
        if config.options.with_filename {
            println!("{}:{}", config.filename, count);
        } else {
            println!("{}", count);
        }
        return Ok(count > 0);
    }

    if matches.is_empty() {
        return Ok(false);
    }

    if config.options.quiet {
        return Ok(true);
    }

    for matched in matches {
        println!("{}", format_match_line(&config, &matched));
    }

    Ok(true)
}

fn format_match_line(config: &Config, matched: &MatchLine<'_>) -> String {
    let line = if config.options.line_number {
        format!("{}:{}", matched.line_no, matched.text)
    } else {
        matched.text.to_string()
    };

    if config.options.with_filename {
        format!("{}:{}", config.filename, line)
    } else {
        line
    }
}

pub fn search<'a>(contents: &'a str, config: &Config) -> Vec<MatchLine<'a>> {
    let mut results = Vec::new();
    let query = &config.query;

    let ignore_case = config.options.ignore_case;
    let invert_match = config.options.invert_match;
    let line_regexp = config.options.line_regexp;
    let quiet = config.options.quiet;
    let max_count = config.options.max_count;

    let query = if ignore_case {
        query.to_lowercase()
    } else {
        query.to_string()
    };

    let normalize_case = |s: &str| {
        if ignore_case {
            s.to_lowercase()
        } else {
            s.to_string()
        }
    };

    let check_line = |line: &str| {
        if line_regexp {
            normalize_case(line) == query
        } else {
            normalize_case(line).contains(&query)
        }
    };

    let mut count = 0;
    for (index, line) in contents.lines().enumerate() {
        if check_line(line) ^ invert_match {
            results.push(MatchLine {
                line_no: index + 1,
                text: line,
            });
            count += 1;
            if quiet || max_count.is_some_and(|max| count >= max) {
                break;
            }
        }
    }
    results
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn case_sensitive() {
        let query = "duct";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
Duct tape.";

        assert_eq!(
            vec![MatchLine {
                line_no: 2,
                text: "safe, fast, productive.",
            }],
            search(
                contents,
                &Config {
                    query: query.to_string(),
                    filename: String::new(),
                    options: Options {
                        ignore_case: false,
                        ..Default::default()
                    },
                }
            )
        );
    }
}
