use std::{
    fs::{self, DirEntry},
    io,
    path::{Path, PathBuf},
};

use anyhow::Result;
use clap::Clap;

const GIT_FOLDER: &str = "git"; // TODO reset to .git

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
    Commit,
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
            let git_path = args
                .path
                .unwrap_or(std::env::current_dir()?)
                .join(GIT_FOLDER);
            fs::create_dir_all(git_path.join("objects"))?;
            fs::create_dir_all(git_path.join("refs"))?;
            println!("Initialized git_rs repository in {}", git_path.display());
        }
        Commands::Commit => {
            let root_path = std::env::current_dir()?;
            let git_path = root_path.join(GIT_FOLDER);
            let objects_path = git_path.join("objects");
            let db = git_rs::Database { path: objects_path };

            visit_dirs(root_path.as_path(), &|file| {
                let file_path = &file.path();
                let data =
                    fs::read(file_path).expect(&format!("Failed to read {}", file_path.display()));
                let blob = git_rs::Object::Blob(data);

                db.store(blob)?;

                Ok(())
            })?;
        }
    }

    Ok(())
}

/// Visits all the files in a directory recursively
fn visit_dirs(dir: &Path, cb: &dyn Fn(&DirEntry) -> Result<()>) -> Result<()> {
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            // TODO implement .gitignore
            match path.file_name().unwrap().to_str().unwrap() {
                ".git" | GIT_FOLDER | "target" => continue,
                _ => (),
            }
            if path.is_dir() {
                visit_dirs(&path, cb)?;
            } else {
                cb(&entry)?;
            }
        }
    }

    Ok(())
}
