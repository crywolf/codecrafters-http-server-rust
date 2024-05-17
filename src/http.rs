use bytes::BytesMut;

#[allow(clippy::upper_case_acronyms)]
#[derive(PartialEq)]
pub enum Method {
    GET,
    POST,
}

impl Method {
    pub fn from(str: &str) -> anyhow::Result<Method> {
        let method = match str.to_ascii_uppercase().as_str() {
            "GET" => Method::GET,
            "POST" => Method::POST,
            _ => anyhow::bail!("unsupported method {}", str),
        };
        Ok(method)
    }
}

#[derive(Debug)]
pub enum Status {
    OK,
    Created,
    BadRequest,
    NotFound,
    MethodNotAllowed,
    InternalServerError,
}

pub mod request {
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::fs;
    use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
    use tokio::net::TcpStream;

    use super::response::Response;
    use super::*;

    #[derive(Debug)]
    pub enum RequestError {
        BadRequestError,
        MethodNotAllowedError,
    }

    impl std::error::Error for RequestError {}

    impl std::fmt::Display for RequestError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self)
        }
    }

    #[allow(dead_code)]
    pub struct Request {
        config: Arc<Config>,
        http_version: String,
        headers: HashMap<String, String>,
        method: Method,
        path: String,
        content: Option<Vec<u8>>,
    }

    impl Request {
        pub async fn new(
            reader: &mut BufReader<TcpStream>,
            config: Arc<Config>,
        ) -> anyhow::Result<Self> {
            let mut request_line = String::new();
            reader.read_line(&mut request_line).await?;

            let parts: Vec<_> = request_line.split(' ').collect();
            if parts.len() != 3 {
                eprintln!("Err: {:?} {:?}", RequestError::BadRequestError, parts);
                anyhow::bail!(RequestError::BadRequestError);
            }

            let method = parts[0];
            let path = parts[1].to_string();
            let http_version = parts[2].to_string();

            let method = match Method::from(method) {
                Ok(method) => method,
                Err(err) => {
                    eprintln!("Err: {:?}", err);
                    anyhow::bail!(RequestError::MethodNotAllowedError);
                }
            };

            let mut headers = HashMap::new();

            let mut lines = reader.lines();
            while let Some(line) = lines.next_line().await? {
                if let Some((k, v)) = line.split_once(": ") {
                    headers.insert(k.to_lowercase(), v.to_string());
                }
                if line.is_empty() {
                    break;
                }
            }

            let mut content = None;
            if let Some(content_length) = headers.get("content-length") {
                let mut buf = vec![0u8; content_length.parse()?];
                reader.read_exact(&mut buf).await?;

                content = Some(buf);
            }

            let r = Self {
                config,
                http_version,
                headers,
                method,
                path,
                content,
            };

            Ok(r)
        }

        pub async fn handle(&self) -> Response {
            let encoding = self.headers.get("accept-encoding");

            // GET /
            if self.path == "/" {
                if self.method != Method::GET {
                    return Response::new(Status::MethodNotAllowed);
                }

                return Response::new(Status::OK);
            }

            // GET /echo/*
            if let Some(echo) = self.path.strip_prefix("/echo/") {
                if self.method != Method::GET {
                    return Response::new(Status::MethodNotAllowed);
                }

                return Response::text(echo, encoding);
            }

            // GET /user-agent/
            if self.path.strip_prefix("/user-agent").is_some() {
                if self.method != Method::GET {
                    return Response::new(Status::MethodNotAllowed);
                }

                let agent = match self.headers.get("user-agent") {
                    Some(agent) => agent,
                    None => "User-Agent header is missing",
                };

                return Response::text(agent, encoding);
            }

            // /files/
            if let Some(filename) = self.path.strip_prefix("/files/") {
                let mut filepath: PathBuf;

                let response = match self.method {
                    // GET /files/ => return file
                    Method::GET => {
                        if let Some(filedir) = &self.config.files_dir {
                            filepath = PathBuf::from(filedir);
                            filepath.push(filename);
                        } else {
                            return Response::new(Status::NotFound);
                        }

                        let response = match fs::read(filepath).await {
                            Ok(content) => Response::binary(content, encoding),
                            Err(_) => Response::new(Status::NotFound),
                        };
                        response
                    }
                    // POST /files/ => store file
                    Method::POST => {
                        if let Some(filedir) = &self.config.files_dir {
                            filepath = PathBuf::from(filedir);
                            filepath.push(filename);

                            if let Some(content) = &self.content {
                                if fs::write(filepath, content).await.is_err() {
                                    return Response::new(Status::InternalServerError);
                                }
                            }
                            Response::new(Status::Created)
                        } else {
                            Response::new(Status::InternalServerError)
                        }
                    }
                };

                return response;
            }

            eprintln!("Err: path {} {:?}", self.path, Status::NotFound);
            Response::new(Status::NotFound)
        }
    }

    #[derive(Default)]
    pub struct Config {
        pub files_dir: Option<String>,
    }
}

pub mod response {
    use super::*;

    pub struct Response<'a> {
        status: Status,
        content: Option<Vec<u8>>,
        content_type: &'a str,
        content_length: usize,
        encoding: Encoding,
        bytes: BytesMut,
    }

    #[derive(PartialEq)]
    enum Encoding {
        None,
        Gzip,
    }

    impl Encoding {
        pub fn from(o: Option<&String>) -> Self {
            if let Some(encoding) = o {
                if encoding.contains("gzip") {
                    Self::Gzip
                } else {
                    Self::None
                }
            } else {
                Self::None
            }
        }
    }

    impl<'a> Response<'a> {
        pub fn new(status: Status) -> Self {
            Self {
                status,
                content: None,
                content_type: "",
                content_length: 0,
                encoding: Encoding::None,
                bytes: BytesMut::with_capacity(64),
            }
        }

        pub fn text(content: &'a str, encoding: Option<&'a String>) -> Self {
            let mut r = Self::new(Status::OK);
            r.content_type = "text/plain";
            r.content_length = content.len();
            r.content = Some(content.to_owned().into_bytes());
            r.encoding = Encoding::from(encoding);
            r
        }

        pub fn binary(content: Vec<u8>, encoding: Option<&'a String>) -> Self {
            let mut r = Self::new(Status::OK);
            r.content_type = "application/octet-stream";
            r.content_length = content.len();
            r.content = Some(content);
            r.encoding = Encoding::from(encoding);
            r
        }

        pub fn as_bytes(&mut self) -> &[u8] {
            let status_line = match &self.status {
                Status::OK => Self::STATUS_200_OK,
                Status::Created => Self::STATUS_201_CREATED,
                Status::BadRequest => Self::STATUS_400_BAD_REQUEST,
                Status::NotFound => Self::STATUS_404_NOT_FOUND,
                Status::MethodNotAllowed => Self::STATUS_405_METHOD_NOT_ALLOWED,
                Status::InternalServerError => Self::STATUS_500_INTERNAL_SERVER_ERROR,
            };

            self.bytes.extend_from_slice(b"HTTP/1.1 ");
            self.bytes.extend_from_slice(status_line.as_bytes());

            if let Some(content) = &self.content {
                // Headers
                if self.encoding == Encoding::Gzip {
                    self.bytes.extend_from_slice(b"\r\nContent-Encoding: ");
                    self.bytes.extend_from_slice(b"gzip");
                }
                self.bytes.extend_from_slice(b"\r\nContent-Type: ");
                self.bytes.extend_from_slice(self.content_type.as_bytes());
                self.bytes.extend_from_slice(b"\r\nContent-Length: ");
                self.bytes
                    .extend_from_slice(self.content_length.to_string().as_bytes());
                self.bytes.extend_from_slice(b"\r\n\r\n");
                // Content
                self.bytes.extend_from_slice(content);
            } else {
                // No content
                self.bytes.extend_from_slice(b"\r\n\r\n");
            }

            &self.bytes
        }

        const STATUS_200_OK: &'static str = "200 OK";
        const STATUS_201_CREATED: &'static str = "201 Created";
        const STATUS_400_BAD_REQUEST: &'static str = "400 Bad Request";
        const STATUS_404_NOT_FOUND: &'static str = "404 Not Found";
        const STATUS_405_METHOD_NOT_ALLOWED: &'static str = "405 Method Not Allowed";
        const STATUS_500_INTERNAL_SERVER_ERROR: &'static str = "500 Internal Server Error";
    }
}
