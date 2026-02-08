use std::error::Error;
use std::fs::File;
use std::io::prelude::*;

use crate::Config;

#[derive(Debug, PartialEq, Eq)]
pub struct MatchLine<'a> {
    pub line_no: usize,
    pub text: &'a str,
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
mod tests {
    use crate::Options;

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
}
