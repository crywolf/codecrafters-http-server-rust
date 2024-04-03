use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    str::FromStr,
};

use http::request::{Request, RequestError};
use http::response::Response;
use http::Status;

mod http;

fn main() -> anyhow::Result<()> {
    let listener = TcpListener::bind("127.0.0.1:4221")?;

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("accepted new connection");
                let _ = handle_connection(stream).map_err(|err| eprintln!("Error: {:?}", err));
            }
            Err(e) => {
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}

fn handle_connection(mut stream: TcpStream) -> anyhow::Result<()> {
    let mut reader = BufReader::new(&mut stream);

    let mut request_line = String::new();
    reader.read_line(&mut request_line)?;

    let request = match Request::from_str(&request_line) {
        Ok(req) => req,
        Err(err) => match err {
            RequestError::BadRequestError => {
                write_response(&mut stream, Response::new(Status::BadRequest))?;
                return Ok(());
            }
            RequestError::MethodNotAllowedError => {
                write_response(&mut stream, Response::new(Status::MethodNotAllowed))?;
                return Ok(());
            }
        },
    };

    let mut response = request.handle();

    Ok(stream.write_all(response.as_bytes())?)
}

pub fn write_response(stream: &mut TcpStream, mut response: Response) -> anyhow::Result<()> {
    Ok(stream.write_all(response.as_bytes())?)
}
