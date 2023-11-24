extern crate crypto;
extern crate rand;
use self::crypto::digest::Digest;
use self::crypto::{sha3::Sha3, aes, blockmodes};
use self::crypto::buffer::{ RefReadBuffer, ReadBuffer, RefWriteBuffer, WriteBuffer, BufferResult };
use std::io::{BufReader, IoSlice, prelude::*, Write};
use std::net::TcpStream;
use common::rkyv::{Deserialize, DeserializeUnsized};
use common::{ClientToServerCommand, ServerToClientResponse, ArchivedServerToClientResponse, FileAndMeta};
use std::str::from_utf8;
use clap::{Parser, Subcommand, command};
use rand::{RngCore, rngs};

const TREE_HEIGHT: usize = 4;

fn send(msg: common::ClientToServerCommand, stream: &mut TcpStream) -> Result<(), std::io::Error> {
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

fn send_str(s: &str, stream: &mut TcpStream) {
    send(common::ClientToServerCommand::Raw(s.to_owned()), stream).unwrap();
}

fn check_hashes_against_tophash(top_hash: String, file_data: &[u8], data_complement_hashes: &Vec<String>) -> bool {
    // Kan inte konvertera till UTF8 då den krypterade datan inte är UTF8.
    // Jag har testat, vi får samma hash som servern, så koden gör det den ska.
    let mut hasher = Sha3::sha3_256();
    hasher.input(file_data);
    let mut hash_val: String = hasher.result_str();
    // println!("{:?}", hash_val);
    // println!("length of comp hashes: {}", data_complement_hashes.len());
    for i in 0..data_complement_hashes.len() {
        hasher.reset();
        hasher.input_str(hash_val.as_str());
        if data_complement_hashes[i].clone() != "empty".to_string() {
            hasher.input_str(data_complement_hashes[i].as_str());
        }
        hash_val = hasher.result_str();
    }
    
    return top_hash == hash_val;
}


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[clap(long, short)]
    password: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Adds files to myapp
    Upload { name: String },
    Ls {
        #[arg(default_value_t = String::from("/"))]
        name: String
    },
    Download { name: String },
    DebugRaw { text: String },
}

fn handle_response(archived: &ArchivedServerToClientResponse, password: String, root_hash: String) {
    match archived {
        ArchivedServerToClientResponse::File(file_and_meta_bytes, merkle_hashes) => {
            // TODO: Add verification.
            let file_and_meta = common::rkyv::check_archived_root::<FileAndMeta>(&file_and_meta_bytes).unwrap();

            // Lite osäker på hur man hanterar dessa.
            let fm = FileAndMeta {
                name: file_and_meta.name.to_string(),
                data: file_and_meta.data.to_vec(),
            };

            let name: Vec<String> = merkle_hashes.iter().map(|h| h.to_string()).collect();
            let file_data = common::rkyv::to_bytes::<_, 256>(&fm).unwrap();

            if !check_hashes_against_tophash(root_hash, &file_data.to_vec(), &name) {
                println!("Bad hashes, file may not be valid...");
            }

            let data = decrypt_data(file_and_meta.data.to_vec(), password);
            std::io::stdout().write_all(&data).unwrap();
        }
        ArchivedServerToClientResponse::UploadOk(_file, merkle_root_hash) => {
            // println!("File successfully uploaded. New root hash: {:?}", merkle_root_hash);
            // Skriv hash till client/bin/root_hash
            if !std::path::Path::new("bin/").exists() { std::fs::create_dir_all("bin/").unwrap(); }
            std::fs::write("./bin/root_hash", merkle_root_hash.to_string()).unwrap();
        }
        // TODO: Possible improvement: Handle FileListing as well
        // TODO: Handle FileNotFound and others
        _ => { println!("response from server: {:#?}", archived); }
    }
}

fn encrypt_data(data: &[u8], password: &String) -> Vec<u8> {
    let mut init_vector: [u8; 16] = [0; 16];
    let mut rng = rngs::OsRng::default();
    rng.fill_bytes(&mut init_vector);

    let mut encryptor = aes::cbc_encryptor(
        aes::KeySize::KeySize256, 
        password.as_bytes(), 
        &init_vector, 
        blockmodes::PkcsPadding);

    let mut encrypted_data = Vec::<u8>::new();
    let mut buffer = [0; 4096];
    let mut read_buffer = RefReadBuffer::new(data);
    let mut write_buffer = RefWriteBuffer::new(&mut buffer);

    loop {
        let result = encryptor.encrypt(&mut read_buffer, &mut write_buffer, true).unwrap();
        encrypted_data.extend(write_buffer.take_read_buffer().take_remaining().iter().map(|&i| i));

        match result {
            BufferResult::BufferUnderflow => break,
            BufferResult::BufferOverflow => {}
        }
    }

    encrypted_data.append(&mut init_vector.to_vec());

    return encrypted_data;
}

fn decrypt_data(data: Vec<u8>, password: String) -> Vec<u8> {
    let split = data.split_at(data.len() - 16);
    let init_vector = split.1;
    let mut encrypted = split.0;

    let mut decryptor = aes::cbc_decryptor(
        aes::KeySize::KeySize256, 
        password.as_bytes(), 
        &init_vector, 
        blockmodes::PkcsPadding);

    let mut decrypted_data = Vec::<u8>::new();
    let mut buffer = [0; 4096];
    let mut read_buffer = RefReadBuffer::new(&mut encrypted);
    let mut write_buffer = RefWriteBuffer::new(&mut buffer);

    loop {
        let result = decryptor.decrypt(&mut read_buffer, &mut write_buffer, true).unwrap();
        decrypted_data.extend(write_buffer.take_read_buffer().take_remaining().iter().map(|&i| i));
        match result {
            BufferResult::BufferUnderflow => break,
            BufferResult::BufferOverflow => { }
        }
    }

    return decrypted_data;
}

// Example usage:
// cargo run -- --password hi upload funny.txt

fn main() {
    let cli = Cli::parse();

    let mut root_hash = String::from("nothing");

    let f = std::fs::File::open("./bin/root_hash");
    if f.is_ok() {
        let mut content = String::new();
        f.unwrap().read_to_string(&mut content).unwrap();
        if !content.is_empty() { root_hash = content; }
    }

    let to_server = match cli.command {
        Commands::Upload { name } => {
            // TODO: Encrypt data before upload
            let data = std::fs::read(&name).unwrap();
            let encrypted = encrypt_data(&data, &cli.password);
            ClientToServerCommand::Upload(name, encrypted)
        }
        Commands::Ls { name } => {
            ClientToServerCommand::ListFiles(name)
        }
        Commands::Download { name } => {
            ClientToServerCommand::Get(name)
        }
        Commands::DebugRaw { text } => {
            ClientToServerCommand::Raw(text)
        }
    };

    // kolla här: ska vi en funktion som täcker alla typer av meddeledanden som klienten
    // kan skicka? eller kanske en för send_file(file), read_file(file_number) ?
    match TcpStream::connect("127.0.0.1:8383") {
        Ok(mut s) => {
            let mut reader = s.try_clone().unwrap();
            let mut buf_reader = BufReader::new(&mut reader);
            send(to_server, &mut s).unwrap();
            //send_str("I am a message.", &mut s);

            // loop {
                let mut header_info = [0u8; 16];
                buf_reader.read_exact(&mut header_info).unwrap();
                let bytes_to_read = usize::from_str_radix(from_utf8(&header_info).unwrap(), 16).unwrap();

                let mut data = vec![0u8; bytes_to_read]; // Skicka inte absurdt stora filer på 32-bit system.
                buf_reader.read_exact(&mut data).unwrap();

                // Avkommentera för att se skickat data i form av fil:
                //std::fs::File::create("output.dat").unwrap().write_all(&data).unwrap();

                let archived = common::rkyv::check_archived_root::<ServerToClientResponse>(&data[..]).unwrap();
                handle_response(archived, cli.password, root_hash);
                // break
            // }

            //send_str("I am another message.", &mut s);
            //send(common::ClientToServerCommand::List("/".into()), &mut s).unwrap();

            // Skicka en stor fil.
            // let large_file: String = fs::read_to_string("text").expect("penguin");
            // send_str(&large_file.as_str(), &mut s);
        }

        Err(_) => { println!("Couldn't connect."); }
    }
}
