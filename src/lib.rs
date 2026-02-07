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

        for arg in &args[3..] {
            match arg.as_str() {
                "-i" | "--ignore-case" => options.ignore_case = true,
                "-n" | "--line-number" => options.line_number = true,
                "-v" | "--invert-match" => options.invert_match = true,
                "-x" | "--line-regexp" => options.line_regexp = true,
                "-H" | "--with-filename" => options.with_filename = true,
                "-h" | "--no-filename" => options.with_filename = false,
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

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let mut f = File::open(config.filename.clone())?;

    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("something went wrong reading the file");

    let results = search(&contents, config);

    for line in results {
        println!("{}", line);
    }

    Ok(())
}

pub fn search<'a>(contents: &'a str, config: Config) -> Vec<String> {
    let mut results = Vec::new();
    let query = &config.query;

    let ignore_case = config.options.ignore_case;
    let line_number = config.options.line_number;
    let invert_match = config.options.invert_match;
    let line_regexp = config.options.line_regexp;

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
                Config {
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
