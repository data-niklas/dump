use std::{fmt::Display, io, path::PathBuf, time::Duration};

use crate::util::create_connection;

use clap::{Args, Command, Parser, Subcommand, ValueEnum};
use clap_complete::{generate, Generator, Shell};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

pub fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
}

#[derive(Subcommand)]
pub enum Commands {
    Clean {
        #[arg(short, long, env)]
        data_directory: PathBuf,
    },
    Stats {
        #[arg(short, long, env)]
        data_directory: PathBuf,
    },
    Serve(ServeArgs),

    Generate {
        shell: Shell,
    },
}

#[derive(Clone, Debug, Serialize)]
pub enum ContentDisposition {
    Inline,
    Attachment,
}

impl Display for ContentDisposition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentDisposition::Inline => write!(f, "inline"),
            ContentDisposition::Attachment => write!(f, "attachment"),
        }
    }
}

impl ValueEnum for ContentDisposition {
    fn from_str(input: &str, _ignore_case: bool) -> Result<Self, String> {
        match input {
            "inline" => Ok(ContentDisposition::Inline),
            "attachment" => Ok(ContentDisposition::Attachment),
            _ => Err(format!("Invalid value for ContentDisposition: {}", input)),
        }
    }

    fn value_variants<'a>() -> &'a [Self] {
        &[ContentDisposition::Inline, ContentDisposition::Attachment]
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        match self {
            ContentDisposition::Inline => Some(clap::builder::PossibleValue::new("inline")),
            ContentDisposition::Attachment => Some(clap::builder::PossibleValue::new("attachment")),
        }
    }
}

fn parse_duration(s: &str) -> Result<Duration, humantime::DurationError> {
    humantime::parse_duration(s)
}

#[derive(Args, Clone, Serialize)]
pub struct ServeArgs {

    // Not needed for the settings command
    #[serde(skip_serializing)]
    #[arg(short, long, env)]
    pub url: String,

    // Should not be exposed in the settings
    #[serde(skip_serializing)]
    #[arg(short, long, env)]
    pub data_directory: PathBuf,

    #[arg(long, env, default_value_t = usize::MAX)]
    pub disk_quota: usize,

    #[serde(skip_serializing)]
    #[arg(short, long, env)]
    pub address: String,

    #[arg(long, env, default_value_t = 256*1024*1024)]
    pub max_size: usize,

    #[arg(long, env, default_value_t = 30 * 24 * 60 * 60 * 1000)]
    pub min_expires: usize,

    #[arg(long, env, default_value_t = 365 * 24 * 60 * 60 * 1000)]
    pub max_expires: usize,

    #[arg(long, env, value_parser, num_args = 0.., value_delimiter = ',', default_values_t = vec!["executable".to_string()])]
    pub blocked_groups: Vec<String>,

    #[serde(skip_serializing)]
    #[arg(long, env)]
    pub blocked_ips: Option<PathBuf>,

    #[arg(long, env, default_value_t = ContentDisposition::Inline)]
    pub content_disposition: ContentDisposition,

    #[arg(long, env, default_value_t = 1)]
    pub rate_limit_count: u64,

    #[arg(long, env, default_value = "5s", value_parser=parse_duration)]
    pub rate_limit_duration: Duration,
}

impl ServeArgs {
    pub fn create_connection(&self) -> Result<Connection, rusqlite::Error> {
        create_connection(&self.data_directory)
    }
}
