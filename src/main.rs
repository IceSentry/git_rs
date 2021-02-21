use std::{env, io::Write, path::PathBuf};

use anyhow::Result;
use clap::Clap;
use git_rs::{object, repository};

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
    CatFile(CatFile),
    Checkout,
    Commit,
    HashObject(HashObject),
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

/// Provide content of repository objects
#[derive(Clap)]
struct CatFile {
    /// Specify the type
    #[clap(name = "TYPE")]
    r#type: object::Type,

    /// The object to display
    #[clap(name = "OBJECT")]
    object: String,
}

#[derive(Clap)]
struct HashObject {
    /// Specify the type
    #[clap(name = "TYPE", short, default_value = "blob")]
    r#type: object::Type,

    /// Actually write the object into the database
    #[clap(name = "write", short)]
    write: bool,

    /// Read object from <file>
    #[clap(parse(from_os_str))]
    path: PathBuf,
}

fn main() -> Result<()> {
    let opts: Opts = Opts::parse();

    match opts.commands {
        Commands::Add => {}
        Commands::CatFile(args) => {
            let repo = repository::find(&env::current_dir()?)?;
            let object = object::read(&repo, object::find(&repo, &args.object, args.r#type))?;
            std::io::stdout().write_all(object.serialize())?;
        }
        Commands::Checkout => {}
        Commands::Commit => {}
        Commands::HashObject(args) => {}
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
