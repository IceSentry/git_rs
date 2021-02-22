use std::{env, io::Write, path::PathBuf};

use anyhow::Result;
use clap::Clap;

/// The stupid content tracker
#[derive(Clap)]
#[clap()]
struct Opts {
    #[clap(subcommand)]
    commands: Commands,
}

#[derive(Clap)]
enum Commands {
    Init(Init),
}

#[derive(Clap)]
struct Init {
    /// Where to create the repository.
    #[clap(name = "directory", parse(from_os_str))]
    path: Option<PathBuf>,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    match opts.commands {
        Commands::Init(args) => {
            let git_path = args.path.unwrap_or(std::env::current_dir()?).join("git"); // TODO reset to .git
            std::fs::create_dir_all(git_path.join("objects"))?;
            std::fs::create_dir_all(git_path.join("refs"))?;
            println!(
                "Initialized git_rs repository in {}",
                git_path.to_string_lossy()
            );
        }
    }

    Ok(())
}
