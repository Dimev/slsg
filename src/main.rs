mod api;
mod cmd;

use clap::Parser;
use cmd::generate::Site;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    /// directory to the site.lua file of the site to build, current working directory by default
    #[clap(short, long)]
    dir: Option<PathBuf>,

    /// directory to write to, public/ by default
    #[clap(short, long)]
    output: Option<PathBuf>,
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    let site = Site::generate(args.dir)?;

    site.write_to_directory::<PathBuf>(None)?;

    Ok(())
}
