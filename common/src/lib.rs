use rkyv::{Archive, Deserialize, Serialize};
pub use rkyv as rkyv;


#[derive(Archive, Deserialize, Serialize)]
#[archive(check_bytes)]
#[archive_attr(derive(Debug))]
pub enum ServerToClientResponse {
    UploadOk(String),
    UploadFailed(String, String),
    FileNotFound(String),
    File(Vec<u8>),
    FileListing(Vec<String>),
    Raw(String),
    UnknownCommand(String),
}

#[derive(Archive, Deserialize, Serialize)]
#[archive(check_bytes)]
#[archive_attr(derive(Debug))]
pub enum ClientToServerCommand {
    Get(String),
    Upload(String, Vec<u8>),
    List(String),
    Raw(String),
}

#[derive(Archive, Deserialize, Serialize)]
#[archive(check_bytes)]
#[archive_attr(derive(Debug))]
pub struct FileAndMeta {
    pub name: String,
    pub data: Vec<u8>,
}
