use std::io::{BufReader, IoSlice, prelude::*};
use std::net::{TcpListener, TcpStream, Shutdown, SocketAddr};
use common::{ClientToServerCommand, ServerToClientResponse, ArchivedClientToServerCommand};
use std::str::from_utf8;

fn send(msg: common::ServerToClientResponse, stream: &mut TcpStream) -> Result<(), std::io::Error> {
    let packet = common::rkyv::to_bytes::<_, 256>(&msg).unwrap();
    let header = format!("{:016X}", packet.len());
    stream.write_vectored(&[
        // header
        IoSlice::new(header.as_bytes()),

        // packet
        IoSlice::new(packet.as_slice()),
    ])?;

    Ok(())
}


fn recieve(mut s: TcpStream) {
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
        handle_command(peer, &mut tcp_writer, archived);
    }

    println!("Disconnected > {peer}");
    s.shutdown(Shutdown::Both).unwrap();
}

fn handle_command(peer: SocketAddr, s: &mut TcpStream, cmd: &ArchivedClientToServerCommand) {
    match cmd {
        ArchivedClientToServerCommand::Raw(msg) => {
            println!("{}: {}", peer, msg);
            send(ServerToClientResponse::Raw(msg.to_string()), s).unwrap();
        }
        _ => {
            println!("no handler for: {:#?}", cmd);
            let mut cmd_name = format!("{:?}", cmd);
            cmd_name.truncate(cmd_name.find('(').unwrap_or(cmd_name.len()));
            send(ServerToClientResponse::UnknownCommand(cmd_name), s).unwrap();
        }
    }
}


fn main() {
    let listener = TcpListener::bind("0.0.0.0:8383").unwrap();

    for s in listener.incoming() {
        match s {
            Ok(s) => { std::thread::spawn(move || recieve(s)); }
            Err(e) => { println!("Error: {}", e); }
        }
    }
}
