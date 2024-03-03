use clap::{Parser, CommandFactory};
use clean::clean;
use opts::Cli;
use serve::serve;

use crate::opts::print_completions;
mod block_list;
mod clean;
mod mime;
mod models;
mod opts;
mod serve;
mod util;

#[tokio::main]
async fn main() {
    let cli: Cli = opts::Cli::parse();
    match cli.command {
        opts::Commands::Clean { data_directory } => clean(data_directory).await,
        opts::Commands::Serve(args) => serve(args).await,
        opts::Commands::Generate { shell } => {
            let mut cmd = Cli::command_for_update();
            print_completions(shell, &mut cmd);
        }
    };
}
