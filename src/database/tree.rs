use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    path::PathBuf,
};

use is_executable::IsExecutable;

use crate::{
    database::{self, Mode, Object},
    ObjectId,
};

pub enum TreeEntry {
    Tree(Tree),
    Entry(Entry),
}

impl TreeEntry {
    pub fn mode(&self) -> &str {
        match self {
            TreeEntry::Tree(_) => Mode::Directory.into(),
            TreeEntry::Entry(entry) => entry.mode(),
        }
    }

    pub fn object_id(&self) -> ObjectId {
        match self {
            // TODO this shouldn't be computed here
            TreeEntry::Tree(tree) => database::hash(&tree.serialize()),
            TreeEntry::Entry(entry) => entry.object_id.clone(),
        }
    }
}

impl std::fmt::Debug for TreeEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TreeEntry::Tree(tree) => write!(f, "{:#?}", tree.entries),
            TreeEntry::Entry(entry) => {
                write!(f, "{} {}", entry.path.display(), &entry.object_id[..6])
            }
        }
    }
}

pub struct Tree {
    entries: HashMap<OsString, TreeEntry>,
}

impl Object for Tree {
    fn serialize_type(&self) -> &str {
        "tree"
    }

    fn serialize_data(&self) -> Vec<u8> {
        self.entries
            .iter()
            .flat_map(|(path, entry)| {
                let mut entry_vec = format!(
                    "{} {}\0",
                    entry.mode(),
                    path.to_str().expect("Failed to convert to str")
                )
                .as_bytes()
                .to_vec();
                entry_vec.extend_from_slice(entry.object_id().as_bytes());
                entry_vec
            })
            .collect()
    }
}

impl Default for Tree {
    fn default() -> Self {
        Self::new()
    }
}

impl Tree {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    fn add_entry(&mut self, parents: &[&OsStr], entry: Entry) {
        if parents.is_empty() {
            self.entries.insert(
                entry
                    .path
                    .file_name()
                    .expect("Failed to read file_name for entry")
                    .into(),
                TreeEntry::Entry(entry.clone()),
            );
        } else {
            let tree_entry = self
                .entries
                .entry(parents[0].into())
                .or_insert_with(|| TreeEntry::Tree(Tree::new()));
            if let TreeEntry::Tree(tree_entry) = tree_entry {
                tree_entry.add_entry(&parents[1..], entry)
            } else {
                panic!("tree_entry is never supposed to be an Entry")
            }
        }
    }

    pub fn traverse(&self, block: &dyn Fn(&Self) -> ObjectId) -> ObjectId {
        for entry in self.entries.values() {
            if let TreeEntry::Tree(tree) = entry {
                tree.traverse(block);
            }
        }
        block(self)
    }
}

pub fn build(entries: &[Entry]) -> Tree {
    let mut entries = entries.to_vec();
    entries.sort_by_key(|x| x.path.clone());

    let mut root = Tree::new();
    for entry in entries {
        let parents = entry.path.parent().map_or_else(Vec::new, |parent| {
            parent.components().map(|c| c.as_os_str()).collect()
        });
        root.add_entry(&parents, entry.clone());
    }
    log::debug!("{:#?}", root.entries);
    root
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub object_id: ObjectId,
}

impl Entry {
    pub fn new(path: PathBuf, object_id: ObjectId) -> Self {
        Self { path, object_id }
    }

    pub fn mode(&self) -> &str {
        if self.path.is_executable() {
            Mode::Executable.into()
        } else {
            Mode::Regular.into()
        }
    }
}
