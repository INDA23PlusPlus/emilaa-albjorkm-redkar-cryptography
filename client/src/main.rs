extern crate crypto;
use self::crypto::digest::Digest;
use self::crypto::sha3::Sha3;
use std::io::{BufReader, IoSlice, prelude::*};
use std::net::TcpStream;
use std::io::Write;
use common::{ClientToServerCommand, ServerToClientResponse, ArchivedServerToClientResponse, FileAndMeta};
use std::str::from_utf8;
use clap::{Parser, Subcommand, command};

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

fn check_hashes_against_tophash(top_hash: String, data_complement_hashes: Vec<String>) -> bool {
    //kolla här
    let mut hasher = Sha3::sha3_256();
    let mut hash_val: String = data_complement_hashes[0].clone();
    for i in 1..data_complement_hashes.len() {
        hasher.reset();
        hasher.input_str(hash_val.as_str());
        hasher.input_str(data_complement_hashes[i].as_str());
        hash_val = hasher.result_str();
    }
    return top_hash == hash_val;
}


#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
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

fn handle_response(archived: &ArchivedServerToClientResponse) {
    match archived {
        ArchivedServerToClientResponse::File(file_and_meta_bytes) => {
           // TODO: Add verification, and encryption!
           let file_and_meta = common::rkyv::check_archived_root::<FileAndMeta>(file_and_meta_bytes).unwrap();
           std::io::stdout().write_all(&file_and_meta.data).unwrap();
        }
        _ => { println!("response from server: {:#?}", archived); }
    }
}

fn main() {
    let cli = Cli::parse();

    let to_server = match cli.command {
        Commands::Upload { name } => {
            let data = std::fs::read(&name).unwrap();
            ClientToServerCommand::Upload(name, data)
        }
        Commands::Ls { name } => {
            ClientToServerCommand::List(name)
        }
        Commands::Download { name } => {
            ClientToServerCommand::Get(name)
        }
        Commands::DebugRaw { text } => {
            ClientToServerCommand::Raw(text)
        }
    };

    let root_hash: String = String::from("nothing");
    // kolla här: ska vi en funktion som täcker alla typer av meddeledanden som klienten
    // kan skicka? eller kanske en för send_file(file), read_file(file_number) ?
    match TcpStream::connect("127.0.0.1:8383") {
            Ok(mut s) => {
            let mut reader = s.try_clone().unwrap();
            let mut buf_reader = BufReader::new(&mut reader);
            send(to_server, &mut s).unwrap();
            //send_str("I am a message.", &mut s);

            loop {
                let mut header_info = [0u8; 16];
                buf_reader.read_exact(&mut header_info).unwrap();
                let bytes_to_read = u64::from_str_radix(from_utf8(&header_info).unwrap(), 16).unwrap();

                let mut data = vec![0u8; bytes_to_read as usize]; // Skicka inte absurdt stora filer på 32-bit system.
                buf_reader.read_exact(&mut data).unwrap();

                // Avkommentera för att se skickat data i form av fil:
                //std::fs::File::create("output.dat").unwrap().write_all(&data).unwrap();

                let archived = common::rkyv::check_archived_root::<ServerToClientResponse>(&data[..]).unwrap();
                handle_response(archived);
                break
            }

            //send_str("I am another message.", &mut s);
            //send(common::ClientToServerCommand::List("/".into()), &mut s).unwrap();

            // Skicka en stor fil.
            // let large_file: String = fs::read_to_string("text").expect("penguin");
            // send_str(&large_file.as_str(), &mut s);
        }

        Err(_) => { println!("Couldn't connect."); }
    }
}
