use minigrep::Config;
use std::env;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    let config = Config::new(&args).unwrap_or_else(|err| {
        eprintln!("Problem parsing arguments: {}", err);
        process::exit(2);
    });

    let found = minigrep::run(config).unwrap_or_else(|err| {
        eprintln!("Application error: {}", err);
        process::exit(2);
    });

    process::exit(if found { 0 } else { 1 });
}
