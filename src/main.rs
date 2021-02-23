use std::{fs, io::Read, path::PathBuf};

use anyhow::Result;
use clap::Clap;
use git_rs::Entry;
use glob::glob;

use flate2::read::{GzDecoder, ZlibDecoder};
use flate2::write::ZlibEncoder;
use flate2::Compression;
use std::io::prelude::*;

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

            let entries: Vec<Entry> = glob("*")
                .expect("Failed to read glob pattern")
                .filter_map(|path| match path {
                    Ok(path) => {
                        if path.is_dir() {
                            None
                        } else {
                            match path.file_name().unwrap().to_str().unwrap() {
                                ".git" | GIT_FOLDER | "target" => None,
                                _ => Some(path),
                            }
                        }
                    }
                    _ => None,
                })
                .map(|path| {
                    let data = fs::read(&path)
                        .unwrap_or_else(|_| panic!("Failed to read {}", &path.display()));
                    let blob = git_rs::Object::Blob(data);
                    let object_id = db.store(blob).expect("Failed to store object in database");
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
            let compressed_file = fs::read(args.path)?;
            let mut d = ZlibDecoder::new(&compressed_file[..]);
            let mut s = String::new();
            d.read_to_string(&mut s).expect("Failed to read to string");
            println!("{}", s);
        }
        Commands::Clear => {
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
