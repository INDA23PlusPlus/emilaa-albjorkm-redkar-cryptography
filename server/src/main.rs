use std::io::{BufReader, prelude::*};
use std::net::{TcpListener, TcpStream, Shutdown};

fn recieve(mut s: TcpStream) {
    loop {
        let buf_reader = BufReader::new(&mut s);
        let data: String = buf_reader
                           .lines()
                           .map(|r| r.unwrap())
                           .take_while(|l| !l.is_empty())
                           .collect::<Vec<_>>().join("\n");
        
        if data.eq("0x0") { break; } // Byt ut det här.

        println!("From {} >> {}", s.peer_addr().unwrap(), data);
    }

    println!("Disconnected: {}", s.peer_addr().unwrap());
    s.shutdown(Shutdown::Both).unwrap();
}

fn main() {
    let listener = TcpListener::bind("0.0.0.0:8383").unwrap();

    for s in listener.incoming() {
        match s {
            Ok(s) => {
                println!("Connected: {}", s.peer_addr().unwrap());
                recieve(s);
            }

            Err(e) => { println!("Error: {}", e); }
        }
    }
}
