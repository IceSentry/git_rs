use crate::database::Object;

pub struct Blob {
    data: Vec<u8>,
}

impl Blob {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data }
    }
}

impl Object for Blob {
    fn serialize_type(&self) -> &str {
        "blob"
    }

    fn serialize_data(&self) -> Vec<u8> {
        self.data.clone()
    }
}
