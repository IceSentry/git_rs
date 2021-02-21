use anyhow::{anyhow, bail, Result};
use ini::Ini;
use std::{fs::File, io::Write, path::PathBuf};

pub struct Repository {
    pub worktree: PathBuf,
    pub dir: PathBuf,
}

impl Repository {
    pub fn new(path: PathBuf, force: bool) -> anyhow::Result<Self> {
        let dir = path.join("git");

        if !(force || dir.is_dir()) {
            return Err(anyhow!(
                "Not a git repository: {}",
                path.to_str().expect("Failed to parse path")
            ));
        }

        let config_path = dir.join("config");
        if config_path.exists() && !force {
            let conf = Ini::load_from_file(dir.join(config_path)).expect("Failed to read config");
            let core = conf
                .section(Some("core"))
                .expect("[core] section not found");
            let version: i32 = core
                .get("repositoryformatversion")
                .expect("repositoryformatversion not found in [core]")
                .parse()
                .expect("repositoryformatversion is not a number");

            if version != 0 {
                bail!("Unsuported repositoryformatversion {}", version);
            }
        } else if !force {
            bail!("Configuration file missing");
        }

        Ok(Self {
            worktree: path,
            dir,
        })
    }

    fn create_dir(&self, path: PathBuf) -> Result<()> {
        let path = self.dir.join(path);
        println!("Creating dir at {}", path.to_string_lossy());

        std::fs::create_dir_all(&path)
            .expect(&format!("Failed to create dir {}", path.to_string_lossy()));
        Ok(())
    }

    fn init_config(&self) -> Result<()> {
        let mut conf = Ini::new();

        conf.with_section(Some("core"))
            .set("repositoryformatversion", "0")
            .set("filemode", "false")
            .set("bare", "false");

        conf.write_to_file(self.worktree.join("config"))?;
        Ok(())
    }
}

pub fn init(path: PathBuf) -> Result<()> {
    let repo = Repository::new(path, true).unwrap();
    if repo.worktree.exists() {
        if !repo.worktree.is_dir() {
            bail!("{} is not a directory", repo.worktree.to_string_lossy())
        }
        if dir_is_empty(&repo.worktree) {
            bail!("{} is not empty", repo.worktree.to_string_lossy())
        }
    } else {
        std::fs::create_dir_all(&repo.worktree).expect(&format!(
            "Failed to create dir {}",
            repo.worktree.to_string_lossy()
        ));
    }

    repo.create_dir(PathBuf::from("branches"))?;
    repo.create_dir(PathBuf::from("objects"))?;
    repo.create_dir(PathBuf::from("refs").join("tags"))?;
    repo.create_dir(PathBuf::from("refs").join("heads"))?;

    let mut file = File::create(repo.worktree.join("description"))?;
    file.write_all(b"Unnamed repository; edit this file 'description' to name the repository.\n")?;

    let mut file = File::create(repo.worktree.join("HEAD"))?;
    file.write_all(b"ref: refs/heads/master\n")?;

    repo.init_config()?;

    Ok(())
}

fn dir_is_empty(path: &PathBuf) -> bool {
    path.read_dir()
        .map(|mut i| i.next().is_none())
        .unwrap_or(false)
}

fn find(path: &PathBuf) -> Result<Repository> {
    let path = path.canonicalize()?;

    if path.join(".git").is_dir() {
        return Repository::new(path, false);
    }

    let parent = path.join("..").canonicalize()?;
    if parent == path {
        bail!("No git directory")
    }

    find(&parent)
}
