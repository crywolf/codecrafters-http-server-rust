use std::sync::Arc;

use http::request::{Config, Request, RequestError};
use http::response::Response;
use http::Status;
use tokio::io::AsyncWriteExt;
use tokio::io::BufReader;
use tokio::net::{TcpListener, TcpStream};

mod http;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut cfg = Config::default();
    let mut args = std::env::args();

    while let Some(arg) = args.next() {
        if arg.as_str() == "--directory" {
            cfg.files_dir = args.next();
        }
    }

    let cfg = Arc::new(cfg);

    let listener = TcpListener::bind("127.0.0.1:4221").await?;

    loop {
        let (stream, _) = listener.accept().await?;
        println!("accepted new connection");
        let cfg = Arc::clone(&cfg);
        tokio::spawn(async move {
            handle_connection(stream, cfg)
                .await
                .map_err(|err| eprintln!("Error: {:?}", err))
        });
    }
}

async fn handle_connection(stream: TcpStream, cfg: Arc<Config>) -> anyhow::Result<()> {
    let mut stream = BufReader::new(stream);

    let request = match Request::new(&mut stream, cfg).await {
        Ok(req) => req,
        Err(err) => match err.downcast_ref() {
            Some(RequestError::BadRequestError) => {
                write_response(&mut stream, Response::new(Status::BadRequest)).await?;
                return Ok(());
            }
            Some(RequestError::MethodNotAllowedError) => {
                write_response(&mut stream, Response::new(Status::MethodNotAllowed)).await?;
                return Ok(());
            }
            None => anyhow::bail!(err),
        },
    };

    let mut response = request.handle().await;

    Ok(stream.write_all(response.as_bytes()).await?)
}

pub async fn write_response(
    stream: &mut BufReader<TcpStream>,
    mut response: Response<'_>,
) -> anyhow::Result<()> {
    Ok(stream.write_all(response.as_bytes()).await?)
}
