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

use git_rs::{
    database::{blob::Blob, commit::Commit, tree::Entry, Database},
    index::Index,
    workspace::Workspace,
    Author, Refs, GIT_FOLDER,
};

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
    /// Add file to the index
    Add {
        /// File to add
        #[clap(parse(from_os_str))]
        path: PathBuf,
    },
    Read {
        #[clap(name = "file", parse(from_os_str))]
        path: PathBuf,
    },
    Clear,
}

fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();
    // pretty_env_logger::init();

    let commands: Commands = Commands::parse();

    match commands {
        Commands::Init { path } => {
            let git_path = path.unwrap_or(std::env::current_dir()?).join(GIT_FOLDER);
            fs::create_dir_all(git_path.join("objects"))?;
            fs::create_dir_all(git_path.join("refs"))?;
            log::info!("Initialized git_rs repository in {}", git_path.display());
        }
        Commands::Commit { message } => {
            // FIXME this assumes we are at root of repo
            let root_path = std::env::current_dir()?;
            let git_path = root_path.join(GIT_FOLDER);

            let workspace = Workspace::new(root_path.clone());
            let db = Database::new(git_path.join("objects"));
            let refs = Refs::new(git_path);

            let entries: Vec<Entry> = workspace
                .list_files()?
                .iter()
                .map(|path| {
                    let data =
                        fs::read(&path).expect(&format!("Failed to read {}", &path.display()));
                    let blob = Blob::new(data);
                    let object_id = db.store(&blob).expect("Failed to store object in database");

                    // Make sure the entry path is relative to root and not the full path
                    let rel_path = pathdiff::diff_paths(path, &root_path).unwrap();
                    log::trace!("{} {}", rel_path.display(), object_id);
                    Entry::new(rel_path, object_id)
                })
                .collect();

            let tree = git_rs::database::tree::build(&entries);
            let tree_id = tree.traverse(&|tree| db.store(tree).expect("Failed while saving tree"));

            let author = Author {
                name: std::env::var("GIT_AUTHOR_NAME").expect("GIT_AUTHOR_NAME is undefined"),
                email: std::env::var("GIT_AUTHOR_EMAIL").expect("GIT_AUTHOR_EMAIL is undefined"),
                time: Utc::now(),
            };

            let message = if let Some(value) = message {
                value
            } else {
                let mut message_buf = String::new();
                std::io::stdin()
                    .read_to_string(&mut message_buf)
                    .expect("Failed to read message from stdin");
                message_buf
            };

            let parent = refs.read_head();
            let is_root = parent.is_some();

            let commit = Commit::new(parent, tree_id, author, message.clone());
            let commit_id = db.store(&commit)?;
            refs.update_head(commit_id.clone())?;

            log::info!(
                "[{}{}]  {}",
                if is_root { "(root-commit) " } else { "" },
                commit_id,
                message.lines().next().expect("Failed to read message")
            );
        }
        Commands::Add { path } => {
            // FIXME this assumes we are at root of repo
            log::debug!("adding {} to index", path.display());
            log::debug!("mode: {}", git_rs::database::Mode::Directory);
            let root_path = std::env::current_dir()?;
            let git_path = root_path.join(GIT_FOLDER);

            let workspace = Workspace::new(root_path);
            let db = Database::new(git_path.join("objects"));
            let mut index = Index::new(git_path.join("index"));

            let data = workspace.read(&path);
            let stat = workspace.file_metadata(&path);

            let blob = Blob::new(data);
            let object_id = db.store(&blob)?;

            index.add(&path, object_id, &stat)?;
            index.write_updates()?;
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
