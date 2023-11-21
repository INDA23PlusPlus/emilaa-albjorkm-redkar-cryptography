use rkyv::{Archive, Deserialize, Serialize};
pub use rkyv as rkyv;


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
