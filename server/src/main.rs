extern crate crypto;
use self::crypto::digest::Digest;
use self::crypto::sha3::Sha3;
use std::io::{BufReader, IoSlice, prelude::*};
use std::net::{TcpListener, TcpStream, Shutdown, SocketAddr};
use common::{ClientToServerCommand, ServerToClientResponse, ArchivedClientToServerCommand, FileAndMeta};
use std::str::from_utf8;
use std::sync::{Arc, Mutex};

// leaf count is 8 right now to be debuggable
const LEAF_COUNT: usize = 8;
const EMPTY_STRING: String = String::new();

pub struct Merkle {
    // perfect binary trees have the size leaf_count * 2 - 1, so we are 1-indexing the nodes
    pub tree: [Option<String>; LEAF_COUNT*2],
    // blir kanske förvirrande om trädet är 1-indexerad och files är inte det
    pub files: [Option<Vec<u8>>; LEAF_COUNT+1],
    hasher: Sha3,
    file_count: usize,
}

impl Merkle {
    pub fn make_tree() -> Merkle {
        Merkle {
            tree: Default::default(),
            files: Default::default(),
            hasher: Sha3::sha3_256(),
            file_count: 0 as usize,
        }
    }

    pub fn add_file(&mut self, file: &[u8]) {
        self.hasher.reset();
        self.hasher.input(file);
        self.files[self.file_count + 1] = Some(file.to_vec());
        self.tree[self.file_count + LEAF_COUNT] = Some(self.hasher.result_str());
        self.update_tree();
        self.file_count += 1;
        //self.sendRootHash();
    }

    fn update_tree(&mut self) {
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

    pub fn get_root_hash(&self) -> Result<String, &'static str> {
        match &self.tree[1] {
            Some(s) => Ok(s.to_string()),
            None => Err("No files in merkle tree"),
        }
    }

    fn send_root_hash(&self) {
        // kolla här: vi borde skicka till klienten med den här funktionen
        let root_hash: String = self.tree[1].clone().expect("No root hash");
        todo!();
    }

    fn get_complement_nodes(&self, mut file_id: usize) -> (Vec<Vec<u8>>, Vec<String>) {
        /* kolla här: vill vi skicka fildatan och komplement hasher i separata
         * meddelanden, eller i en och samma?
         * just nu skickar jag fildatan med också
        */
        let mut data_complement_files: Vec<Vec<u8>> = vec![];
        let mut data_complement_hashes: Vec<String> = vec![];
        if let Some(file) = self.files[file_id].clone() {
            data_complement_files.push(file);
        } else {
            panic!("No file with file_id: {}!", file_id);
        }

        // return a vector of complementary nodes from greatest depth to highest
        let mut node = file_id + LEAF_COUNT;
        let mut current_known_hash: usize = node;
        while node > 1 {
            node /= 2;
            for i in 0..2 {
                if let Some(hash) = self.tree[node * 2 + i].clone() {
                    if current_known_hash != node * 2 + i {
                        data_complement_hashes.push(hash);
                    }
                }
            }
            current_known_hash = node;
        }
        return (data_complement_files, data_complement_hashes);
    }
}



fn send(msg: common::ServerToClientResponse, stream: &mut TcpStream) -> Result<(), std::io::Error> {
    let packet = common::rkyv::to_bytes::<_, 256>(&msg).unwrap();
    let header = format!("{:016X}", packet.len());
    stream.write_vectored(&[
        // Header
        IoSlice::new(header.as_bytes()),

        // Packet
        IoSlice::new(packet.as_slice()),
    ])?;

    Ok(())
}


fn recieve(mut s: TcpStream, merkle: Arc<Mutex<Merkle>>) {
    let peer = s.peer_addr().unwrap();
    let mut tcp_writer = s.try_clone().unwrap();
    let mut buf_reader = BufReader::new(&mut s);

    println!("Connected > {peer}");

    loop {
        // Ta emot hex header och ta reda på hur långt meddelandet är.
        let mut header_info = [0u8; 16];
        if buf_reader.read_exact(&mut header_info).is_err() { break; }
        let bytes_to_read = u64::from_str_radix(from_utf8(&header_info).unwrap(), 16).unwrap();

        let mut data = vec![0u8; bytes_to_read as usize]; // Skicka inte absurdt stora filer på 32-bit system.
        buf_reader.read_exact(&mut data).unwrap();

        // Avkommentera för att se skickat data i form av fil:
        //std::fs::File::create("output.dat").unwrap().write_all(&data).unwrap();

        let archived = common::rkyv::check_archived_root::<ClientToServerCommand>(&data[..]).unwrap();
        handle_command(peer, &mut tcp_writer, archived, merkle.clone());
    }

    println!("Disconnected > {peer}");
    s.shutdown(Shutdown::Both).unwrap();
}

fn handle_command(peer: SocketAddr, s: &mut TcpStream, cmd: &ArchivedClientToServerCommand, merkle: Arc<Mutex<Merkle>>) {
    match cmd {
        ArchivedClientToServerCommand::Raw(msg) => {
            println!("{}: {}", peer, msg);
            send(ServerToClientResponse::Raw(msg.to_string()), s).unwrap();
        }
        ArchivedClientToServerCommand::Upload(name, data) => {
            let file_and_meta = FileAndMeta {
                name: name.to_string(),
                data: data.to_vec(),
            };

            let file_bytes = common::rkyv::to_bytes::<_, 256>(&file_and_meta).unwrap();
            merkle.lock().unwrap().add_file(&file_bytes);


            let root_hash = merkle.lock().unwrap().get_root_hash().unwrap();

            send(ServerToClientResponse::UploadOk(name.to_string(), root_hash), s).unwrap();
        }
        ArchivedClientToServerCommand::Get(name) => {
            let merkle = merkle.lock().unwrap();

            // This is really ugly. But unfortunately we do not know where the file is in the tree.
            // An extra B-tree could be used to store this. Or, we could make sure the merkle tree
            // was based around the path separator. But that would be too much work probably.
            let mut file_found = false;
            for i in 0 .. merkle.file_count {
                // Remember that files are not zero indexed in the merkle tree.
                let Some(file) = &merkle.files[i + 1] else {
                    continue
                };
                let archived = common::rkyv::check_archived_root::<FileAndMeta>(file).unwrap();
                if *name != archived.name {
                    continue
                }

                println!("Found requested file: {}", archived.name);
                file_found = true;

                let complements = merkle.get_complement_nodes(i + 1);

                send(ServerToClientResponse::File(complements.0, complements.1), s).unwrap();
                break
            }
            if !file_found {
                send(ServerToClientResponse::FileNotFound(name.to_string()), s).unwrap();
            }
        }
        ArchivedClientToServerCommand::ListFiles(filter) => {
            let merkle = merkle.lock().unwrap();
            let mut files = vec![];
            for i in 0 .. merkle.file_count {
                // Remember that files are not zero indexed in the merkle tree.
                let Some(file) = &merkle.files[i + 1] else {
                    continue
                };
                let archived = common::rkyv::check_archived_root::<FileAndMeta>(file).unwrap();
                if filter.starts_with(archived.name.as_str()) {
                    continue
                }
                files.push(archived.name.to_string());
            }
            send(ServerToClientResponse::FileListing(files), s).unwrap();

        }
        /*_ => {
            println!("No handler for: {:#?}", cmd);
            let mut cmd_name = format!("{:?}", cmd);
            cmd_name.truncate(cmd_name.find('(').unwrap_or(cmd_name.len()));
            send(ServerToClientResponse::UnknownCommand(cmd_name), s).unwrap();
        }*/
    }
}


fn main() {
    let listener = TcpListener::bind("0.0.0.0:8383").unwrap();
    // i hackmd:n så föreslår dem att klienten kan kunna skicka en init signal för servern att skapa merkle trädet. Då liknar servern ett faktiskt server. Vi kan implementera det om ni vill. Just nu så skapar jag bara ett merkle träd här.
    let merkle = Arc::new(Mutex::new(Merkle::make_tree()));

    for s in listener.incoming() {
        match s {
            Ok(s) => {
                let m = merkle.clone();
                std::thread::spawn(move || recieve(s, m));
            }
            Err(e) => { println!("Error: {}", e); }
        }
    }
}
