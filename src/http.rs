use bytes::BytesMut;

#[allow(clippy::upper_case_acronyms)]
#[derive(PartialEq)]
pub enum Method {
    GET,
    // POST,
}

impl Method {
    pub fn from(str: &str) -> anyhow::Result<Method> {
        let method = match str.to_ascii_uppercase().as_str() {
            "GET" => Method::GET,
            //"POST" => Method::POST,
            _ => anyhow::bail!("unsupported method {}", str),
        };
        Ok(method)
    }
}

#[derive(Debug)]
pub enum Status {
    OK,
    BadRequest,
    NotFound,
    MethodNotAllowed,
}

pub mod request {
    use std::collections::HashMap;
    use std::io::BufRead;

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
        http_version: String,
        headers: HashMap<String, String>,
        method: Method,
        path: String,
    }

    impl Request {
        pub fn new(reader: &mut impl BufRead) -> anyhow::Result<Self> {
            let mut request_line = String::new();
            reader.read_line(&mut request_line)?;

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

            for line in reader
                .lines()
                .map(|line| line.unwrap_or_default())
                .take_while(|line| !line.is_empty())
            {
                if let Some((k, v)) = line.split_once(": ") {
                    headers.insert(k.to_owned(), v.to_string());
                }
            }

            let r = Self {
                http_version,
                headers,
                method,
                path,
            };

            Ok(r)
        }

        pub fn handle(&self) -> Response {
            if self.path == "/" {
                return Response::new(Status::OK);
            }

            if let Some(echo) = self.path.strip_prefix("/echo/") {
                return Response::text(echo);
            }

            if self.path.strip_prefix("/user-agent").is_some() {
                let agent = match self.headers.get("User-Agent") {
                    Some(agent) => agent,
                    None => "User-Agent header is missing",
                };
                return Response::text(agent);
            }

            eprintln!("Err: path {} {:?}", self.path, Status::NotFound);
            Response::new(Status::NotFound)
        }
    }
}

pub mod response {
    use super::*;

    pub struct Response<'a> {
        status: Status,
        content: Option<&'a str>,
        content_type: &'a str,
        content_length: usize,
        bytes: BytesMut,
    }

    impl<'a> Response<'a> {
        pub fn new(status: Status) -> Self {
            Self {
                status,
                content: None,
                content_type: "",
                content_length: 0,
                bytes: BytesMut::with_capacity(64),
            }
        }

        pub fn text(content: &'a str) -> Self {
            let mut r = Self::new(Status::OK);
            r.content_type = "text/plain";
            r.content_length = content.len();
            r.content = Some(content);
            r
        }

        pub fn as_bytes(&mut self) -> &[u8] {
            let status_line = match &self.status {
                Status::OK => Self::STATUS_200_OK,
                Status::BadRequest => Self::STATUS_400_BAD_REQUEST,
                Status::NotFound => Self::STATUS_404_NOT_FOUND,
                Status::MethodNotAllowed => Self::STATUS_405_METHOD_NOT_ALLOWED,
            };

            self.bytes.extend_from_slice(b"HTTP/1.1 ");
            self.bytes.extend_from_slice(status_line.as_bytes());

            if let Some(content) = &self.content {
                self.bytes.extend_from_slice(b"\r\nContent-Type: ");
                self.bytes.extend_from_slice(self.content_type.as_bytes());
                self.bytes.extend_from_slice(b"\r\nContent-Length: ");
                self.bytes
                    .extend_from_slice(self.content_length.to_string().as_bytes());
                self.bytes.extend_from_slice(b"\r\n\r\n");
                self.bytes.extend_from_slice(content.as_bytes());
            } else {
                self.bytes.extend_from_slice(b"\r\n\r\n");
            }

            &self.bytes
        }

        const STATUS_200_OK: &'static str = "200 OK";
        const STATUS_400_BAD_REQUEST: &'static str = "400 Bad Request";
        const STATUS_404_NOT_FOUND: &'static str = "404 Not Found";
        const STATUS_405_METHOD_NOT_ALLOWED: &'static str = "405 Method Not Allowed";
    }
}
