mod config;
mod grep;

pub use config::{Config, Options, ParseError};
pub use grep::{MatchLine, run, search};
