use std::error::Error;
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
    pub max_count: Option<usize>,
}

pub struct Config {
    query: String,
    filename: String,
    options: Options,
}

impl Config {
    pub fn new(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 3 {
            return Err("not enough arguments");
        }

        let query = args[1].clone();
        let filename = args[2].clone();

        let mut options = Options::default();
        let mut index = 3;

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
                "-m" | "--max-count" => {
                    if index >= args.len() {
                        return Err("missing value for max-count");
                    }
                    let count_str = &args[index];
                    index += 1;
                    match count_str.parse::<usize>() {
                        Ok(count) => options.max_count = Some(count),
                        Err(_) => return Err("invalid value for max-count"),
                    }
                }
                _ => return Err("unknown argument"),
            }
        }

        Ok(Config {
            query,
            filename,
            options,
        })
    }
}

pub fn run(config: Config) -> Result<bool, Box<dyn Error>> {
    let mut f = File::open(config.filename.clone())?;

    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("something went wrong reading the file");

    let results = search(&contents, &config);

    if results.is_empty() {
        return Ok(false);
    }

    if config.options.quiet {
        return Ok(true);
    }

    for line in results {
        println!("{}", line);
    }

    Ok(true)
}

pub fn search<'a>(contents: &'a str, config: &Config) -> Vec<String> {
    let mut results = Vec::new();
    let query = &config.query;

    let ignore_case = config.options.ignore_case;
    let line_number = config.options.line_number;
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
            let res = if line_number {
                format!("{}:{}", index + 1, line)
            } else {
                format!("{}", line)
            };
            let res = if config.options.with_filename {
                format!("{}:{}", config.filename, res)
            } else {
                res
            };
            results.push(res);
            count += 1;
            if quiet || (max_count.is_some() && count >= max_count.unwrap()) {
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
            vec!["safe, fast, productive."],
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
