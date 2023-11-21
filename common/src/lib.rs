extern crate crypto;
use self::crypto::digest::Digest;
use self::crypto::sha3::Sha3;
use rkyv::{Archive, Deserialize, Serialize};
pub use rkyv as rkyv;

const LEAF_COUNT: usize = 8;
const EMPTY_STRING: String = String::new();

pub struct Merkle {
    pub tree: [Option<String>; LEAF_COUNT*2],
    hasher: Sha3,
    file_count: usize,
}

impl Merkle { 
    pub fn makeTree() -> Merkle { 
        Merkle { 
            tree: Default::default(), 
            hasher: Sha3::sha3_256(),
            file_count: 0 as usize,
        }
    }

    pub fn addFile(&mut self, file: &str) {
        self.hasher.reset();
        self.hasher.input_str(file);
        self.tree[self.file_count + LEAF_COUNT] = Some(self.hasher.result_str());
        self.updateTree();
        self.file_count += 1;
        //self.sendRootHash();
    }
    
    fn updateTree(&mut self) {
        let mut node: usize = (self.file_count+LEAF_COUNT)/2;
        while node >= 1 {
            self.hasher.reset();
            for i in 0..2 {
                if let Some(s) = &self.tree[node * 2 + i] {
                    self.hasher.input_str(s.as_str());
                }
            }
            self.tree[node] = Some(self.hasher.result_str());
            node /= 2;
        }
    }

    pub fn getRootHash(&self) -> Result<String, &'static str> {
        match &self.tree[1] {
            Some(s) => Ok(s.to_string()),
            None => Err("No files in merkle tree"),
        }
    }

    fn sendRootHash(&self) {
        let root_hash: String = self.tree[1].clone().expect("No root hash");
        todo!();
    }
}


#[derive(Archive, Deserialize, Serialize)]
#[archive(check_bytes)]
#[archive_attr(derive(Debug))]
pub enum ServerToClientResponse {
    UploadOk(String),
    UploadFailed(String, String),
    FileNotFound(String),
    File(String),
    FileListing(Vec<String>),
    Raw(String),
    UnknownCommand(String),
}

#[derive(Archive, Deserialize, Serialize)]
#[archive(check_bytes)]
#[archive_attr(derive(Debug))]
pub enum ClientToServerCommand {
    Get(String),
    Upload(String, String),
    List(String),
    Raw(String),
}


