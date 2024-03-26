use std::{
    io::Write,
    net::{TcpListener, TcpStream},
};

fn main() -> std::io::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221").unwrap();

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                handle_connection(stream)?
            }
            Err(e) => {
                eprintln!("error: {}", e);
            }
        }
    }
    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> std::io::Result<()> {
    let response = b"HTTP/1.1 200 OK\r\n\r\n";
    stream.write_all(response)
}
