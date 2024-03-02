use clap::Parser;
use clean::clean;
use opts::Cli;
use serve::serve;

mod mime;
mod util;
mod opts;
mod serve;
mod clean;
mod models;

#[tokio::main]
async fn main() {
    let cli: Cli = opts::Cli::parse();
    match cli.command {
        opts::Commands::Clean { data_directory } => clean(data_directory).await,
        opts::Commands::Serve(args) => serve(args).await
    };
}
