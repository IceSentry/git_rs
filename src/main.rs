#![allow(clippy::expect_fun_call)]

use std::{
    fs,
    io::{Read, Write},
    path::PathBuf,
};

use anyhow::Result;
use clap::Clap;
use flate2::read::ZlibDecoder;
use git_rs::Entry;
use glob::glob;

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
    Read(ReadArgs),
    Clear,
}

#[derive(Clap)]
struct Init {
    /// Where to create the repository.
    #[clap(name = "directory", parse(from_os_str))]
    path: Option<PathBuf>,
}

#[derive(Clap)]
struct ReadArgs {
    /// Where to create the repository.
    #[clap(name = "file", parse(from_os_str))]
    path: PathBuf,
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
                    let blob = git_rs::Object::Blob(data);
                    let object_id = db.store(blob).expect("Failed to store object in database");
                    println!("{} {}", path.display(), object_id);
                    Entry {
                        name: path,
                        object_id,
                    }
                })
                .collect();

            let tree = git_rs::Object::Tree(entries);
            let object_id = db.store(tree)?;

            println!("tree: {}", object_id);
        }
        Commands::Read(args) => {
            // WARN this is just for debug purposes
            let compressed_file = fs::read(args.path)?;
            let mut d = ZlibDecoder::new(&compressed_file[..]);
            let mut buf = Vec::new();
            d.read_to_end(&mut buf)
                .expect("Failed to read compressed_file");
            std::io::stdout().write_all(&buf)?; // This makes it possible to use hexdump
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
