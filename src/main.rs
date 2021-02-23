#![allow(clippy::expect_fun_call)]

use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::Result;
use chrono::Utc;
use clap::Clap;
use dotenv::dotenv;
use flate2::read::ZlibDecoder;
use glob::glob;

use git_rs::database::{Author, Database, Entry, Object};

const GIT_FOLDER: &str = "git"; // TODO reset to .git

/// git_rs a git reimplementation in rust
#[derive(Clap)]
#[clap()]
enum Commands {
    Init {
        /// Where to create the repository.
        #[clap(name = "directory", parse(from_os_str))]
        path: Option<PathBuf>,
    },
    Commit {
        #[clap(short, long)]
        message: Option<String>,
    },
    Read {
        #[clap(name = "file", parse(from_os_str))]
        path: PathBuf,
    },
    Clear,
}

fn main() -> Result<()> {
    dotenv().ok();

    let commands: Commands = Commands::parse();

    match commands {
        Commands::Init { path } => {
            let git_path = path.unwrap_or(std::env::current_dir()?).join(GIT_FOLDER);
            fs::create_dir_all(git_path.join("objects"))?;
            fs::create_dir_all(git_path.join("refs"))?;
            println!("Initialized git_rs repository in {}", git_path.display());
        }
        Commands::Commit { message } => {
            let root_path = std::env::current_dir()?;
            let git_path = root_path.join(GIT_FOLDER);
            let objects_path = git_path.join("objects");
            let db = Database { path: objects_path };

            let entries: Vec<Entry> = glob("**/*")
                .expect("Failed to read glob pattern")
                .filter_map(|path| match path {
                    Ok(path) => {
                        if path.is_dir()
                            || path.starts_with(".git")
                            || path.starts_with(GIT_FOLDER)
                            || path.starts_with("target")
                        {
                            None
                        } else {
                            Some(path)
                        }
                    }
                    _ => None,
                })
                .map(|path| {
                    let data =
                        fs::read(&path).expect(&format!("Failed to read {}", &path.display()));
                    let blob = Object::Blob(data);
                    let object_id = db.store(blob).expect("Failed to store object in database");
                    println!("{} {}", path.display(), object_id);
                    Entry {
                        name: path,
                        object_id,
                    }
                })
                .collect();

            let tree = Object::Tree(entries);
            let tree_id = db.store(tree)?;

            println!("tree: {}", tree_id);

            let name = std::env::var("GIT_AUTHOR_NAME").expect("GIT_AUTHOR_NAME is undefined");
            let email = std::env::var("GIT_AUTHOR_EMAIL").expect("GIT_AUTHOR_EMAIL is undefined");

            let author = Author {
                name,
                email,
                time: Utc::now(),
            };

            let message = if let Some(message) = message {
                message
            } else {
                let mut message_buf = String::new();
                std::io::stdin()
                    .read_to_string(&mut message_buf)
                    .expect("Failed to read message from stdin");
                message_buf
            };

            let commit = Object::Commit {
                tree_id,
                author,
                message: message.clone(),
            };

            let commit_id = db.store(commit)?;
            std::fs::write(git_path.join("HEAD"), &commit_id)?;

            println!(
                "[(root-commit) {}]  {}",
                commit_id,
                message.lines().next().expect("Failed to read message")
            );
        }
        Commands::Read { path } => {
            // WARN this is just for debug purposes
            let compressed_file = fs::read(path)?;
            let mut d = ZlibDecoder::new(&compressed_file[..]);
            let mut buf = Vec::new();
            d.read_to_end(&mut buf)
                .expect("Failed to read compressed_file");
            std::io::stdout().write_all(&buf)?; // This makes it possible to pipe the output
        }
        Commands::Clear => {
            // WARN this is just for debug purposes
            std::fs::remove_dir_all(GIT_FOLDER)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use flate2::{read::ZlibDecoder, write::ZlibEncoder, Compression};
    use std::io::prelude::*;

    #[test]
    fn flate() -> anyhow::Result<()> {
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(b"Hello world")?;
        let compressed = encoder.finish()?;

        std::fs::create_dir_all("./tmp")?;
        let mut file = std::fs::File::create("./tmp/compressed")?;
        file.write_all(&compressed)?;

        let compressed_file = std::fs::read("./tmp/compressed")?;
        let mut decoder = ZlibDecoder::new(&compressed_file[..]);
        let mut s = String::new();
        decoder.read_to_string(&mut s).unwrap();

        std::fs::remove_dir_all("./tmp")?;

        Ok(())
    }
}
