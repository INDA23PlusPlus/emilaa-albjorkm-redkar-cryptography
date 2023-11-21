use std::net::TcpStream;
use std::io::Write;
// use std::fs;

fn send(s: &str, stream: &mut TcpStream) {
    let max_header_size = 16;
    let mut hex_header = format!("{:X}", s.len().to_string().parse::<u64>().unwrap());
    let header_size = max_header_size - hex_header.len();
    hex_header.insert_str(0, "0".repeat(header_size).as_str());
    let message = hex_header + s;

    stream.write_all(message.as_bytes()).unwrap();
}

fn main() {
    match TcpStream::connect("0.0.0.0:8383") {
        Ok(mut s) => {
            send("I am a message.", &mut s);
            send("I am another message.", &mut s);
            // Skicka en stor fil.
            // let large_file: String = fs::read_to_string("text").expect("penguin");
            // send(&large_file.as_str(), &mut s);
        }

        Err(_) => { println!("Couldn't connect."); }
    }
}
