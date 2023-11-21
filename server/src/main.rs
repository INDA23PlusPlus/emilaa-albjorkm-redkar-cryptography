use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream, Shutdown};
use common::{ClientToServerCommand, ServerToClientResponse};
use std::str::from_utf8;

fn recieve(mut s: TcpStream) {
    let peer = s.peer_addr().unwrap();
    let mut buf_reader = BufReader::new(&mut s);
    
    println!("Connected > {peer}");

    loop {
        // Ta emot hex header och ta reda på hur långt meddelandet är.
        let mut header_info = [0u8; 16];
        if buf_reader.read_exact(&mut header_info).is_err() { break; }
        let bytes_to_read = u64::from_str_radix(from_utf8(&header_info).unwrap(), 16).unwrap();

        let mut data = vec![0u8; bytes_to_read as usize]; // Skicka inte absurdt stora filer på 32-bit system.
        buf_reader.read_exact(&mut data).unwrap();

        println!("{}: {}", peer, from_utf8(&data).unwrap());
    }

    println!("Disconnected > {peer}");
    s.shutdown(Shutdown::Both).unwrap();
}

fn main() {

    // TODO: This is just an example of how to use rkyv. Remove later!
    let value = ServerToClientResponse::UploadOk("funny.txt".into());
    let response = common::rkyv::to_bytes::<_, 256>(&value).unwrap();
    println!("Response is: {:#?}", response);
    let archived = common::rkyv::check_archived_root::<ServerToClientResponse>(&response[..]).unwrap();
    use common::ArchivedServerToClientResponse::UploadOk;
    if let UploadOk(v) = archived {
        println!("{:#?}", v);
    }

    let listener = TcpListener::bind("0.0.0.0:8383").unwrap();

    for s in listener.incoming() {
        match s {
            Ok(s) => { std::thread::spawn(move || recieve(s)); }

            Err(e) => { println!("Error: {}", e); }
        }
    }
}
