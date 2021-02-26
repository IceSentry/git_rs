use crate::{database::Object, Author, ObjectId};

pub struct Commit {
    parent: Option<ObjectId>,
    tree_id: ObjectId,
    author: Author,
    message: String,
}

impl Commit {
    pub fn new(
        parent: Option<ObjectId>,
        tree_id: ObjectId,
        author: Author,
        message: String,
    ) -> Self {
        Self {
            parent,
            tree_id,
            author,
            message,
        }
    }
}

impl Object for Commit {
    fn serialize_type(&self) -> &str {
        "commit"
    }

    fn serialize_data(&self) -> Vec<u8> {
        let mut lines = vec![format!("tree {}", self.tree_id)];
        if let Some(parent_id) = &self.parent {
            lines.push(format!("parent {}", parent_id));
        }
        lines.push(format!("author {}", self.author));
        lines.push(format!("committer {}", self.author));
        lines.push("".into());
        lines.push(self.message.clone());

        lines.join("\n").as_bytes().to_vec()
    }
}
