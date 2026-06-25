mod args;
mod dispatch;

use clap::Parser;
use skillspec::error::Result;

pub fn run() -> Result<()> {
    let cli = args::Cli::parse();
    dispatch::run(cli.command)
}
