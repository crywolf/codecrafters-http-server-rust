use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
};

use anyhow::Context;
use http::{Method, Response, Status};

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

    let parts: Vec<_> = request_line.split(' ').collect();
    if parts.len() != 3 {
        let response = Response::new(Status::BadRequest);
        write_response(&mut stream, response)?
    }

    let method = parts[0];
    let path = parts[1];
    let _http_ver = parts[2];

    if check_method(method).is_err() {
        let response = Response::new(Status::MethodNotAllowed);
        write_response(&mut stream, response)?;
        return Ok(());
    };

    let mut response = hande_path(path);

    Ok(stream.write_all(response.as_bytes())?)
}

pub fn check_method(method: &str) -> anyhow::Result<()> {
    Method::from(method)
        .context("check HTTP method")
        .map_err(|err| {
            eprintln!("Err: {:?}", err);
            err
        })?;
    Ok(())
}

pub fn hande_path(path: &str) -> Response {
    if path == "/" {
        return Response::new(Status::OK);
    }

    if let Some(echo) = path.strip_prefix("/echo/") {
        return Response::text(echo);
    }

    eprintln!("Err: path {path} {:?}", Status::NotFound);
    Response::new(Status::NotFound)
}

pub fn write_response(stream: &mut TcpStream, mut response: Response) -> anyhow::Result<()> {
    Ok(stream.write_all(response.as_bytes())?)
}
