use std::path::PathBuf;

use crate::util::create_connection;

use clap::{Args, Parser, Subcommand};
use rusqlite::Connection;

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    Clean {
        #[arg(short, long, env)]
        data_directory: PathBuf,
    },
    Serve(ServeArgs),
}

#[derive(Args, Clone)]
pub struct ServeArgs {
    #[arg(short, long, env)]
    pub url: String,
    #[arg(short, long, env)]
    pub data_directory: PathBuf,
    #[arg(long, env, default_value_t = usize::MAX)]
    pub disk_quota: usize,
    #[arg(short, long, env)]
    pub address: String,
    #[arg(long, env, default_value_t = 256*1024*1024)]
    pub max_size: usize,
    #[arg(long, env, default_value_t = 30 * 24 * 60 * 60 * 1000)]
    pub min_expires: usize,
    #[arg(long, env, default_value_t = 365 * 24 * 60 * 60 * 1000)]
    pub max_expires: usize,
    #[arg(long, env, value_parser, num_args = 0.., value_delimiter = ',', default_values_t = vec!["executable".to_string()])]
    pub denied_groups: Vec<String>,
}

impl ServeArgs {
    pub fn create_connection(&self) -> Result<Connection, rusqlite::Error> {
        create_connection(&self.data_directory)
    }
}
