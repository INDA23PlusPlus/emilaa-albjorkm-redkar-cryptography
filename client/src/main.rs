use std::net::TcpStream;
use std::io::Write;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    match TcpStream::connect("0.0.0.0:8383") {
        Ok(mut s) => {
            s.write(b"I am a message.\n\n").unwrap();
            sleep(Duration::new(2, 0));
            s.write(b"I am another message.\n\n").unwrap();
            sleep(Duration::new(2, 0));
            s.write(b"0x0\n\n").unwrap(); // Byt ut det här.
        }

        Err(_) => { println!("Couldn't connect."); }
    }
}
