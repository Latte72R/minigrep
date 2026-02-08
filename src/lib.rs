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
    query: String,
    filenames: Vec<String>,
    options: Options,
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

#[derive(Debug, PartialEq, Eq)]
pub struct MatchLine<'a> {
    pub line_no: usize,
    pub text: &'a str,
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

pub fn run(config: Config) -> Result<bool, Box<dyn Error>> {
    let with_filename = config
        .options
        .with_filename
        .unwrap_or(config.filenames.len() > 1);
    let mut found = false;

    for filename in &config.filenames {
        let mut f = File::open(filename)?;
        let mut contents = String::new();
        f.read_to_string(&mut contents)
            .expect("something went wrong reading the file");

        let matches = search(&contents, &config);

        if config.options.count {
            let count = matches.len();
            if with_filename {
                println!("{}:{}", filename, count);
            } else {
                println!("{}", count);
            }
            found = found || count > 0;
            continue;
        }

        if matches.is_empty() {
            continue;
        }
        found = true;

        if config.options.quiet {
            return Ok(true);
        }

        for matched in matches {
            println!(
                "{}",
                format_match_line(filename, with_filename, &config, &matched)
            );
        }
    }

    Ok(found)
}

fn format_match_line(
    filename: &str,
    with_filename: bool,
    config: &Config,
    matched: &MatchLine<'_>,
) -> String {
    let line = if config.options.line_number {
        format!("{}:{}", matched.line_no, matched.text)
    } else {
        matched.text.to_string()
    };

    if with_filename {
        format!("{}:{}", filename, line)
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
                    filenames: Vec::new(),
                    options: Options {
                        ignore_case: false,
                        ..Default::default()
                    },
                }
            )
        );
    }

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
