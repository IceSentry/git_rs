use std::path::PathBuf;

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
    Add,
    CatFile,
    Checkout,
    Commit,
    HashObject,
    Init(Init),
    Log,
    LsTree,
    Merge,
    Rebase,
    RevParse,
    Rm,
    ShowRef,
    Tag,
}

#[derive(Clap)]
struct Init {
    /// Where to create the repository.
    #[clap(name = "directory", default_value = ".", parse(from_os_str))]
    path: PathBuf,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    match opts.commands {
        Commands::Add => {}
        Commands::CatFile => {}
        Commands::Checkout => {}
        Commands::Commit => {}
        Commands::HashObject => {}
        Commands::Init(init) => git_rs::repository::init(init.path)?,
        Commands::Log => {}
        Commands::LsTree => {}
        Commands::Merge => {}
        Commands::Rebase => {}
        Commands::RevParse => {}
        Commands::Rm => {}
        Commands::ShowRef => {}
        Commands::Tag => {}
    }

    Ok(())
}
