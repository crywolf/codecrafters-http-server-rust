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
        method: Method,
        path: String,
    }

    impl std::str::FromStr for Request {
        type Err = RequestError;

        fn from_str(request_str: &str) -> Result<Self, Self::Err> {
            let parts: Vec<_> = request_str.split(' ').collect();
            if parts.len() != 3 {
                eprintln!("Err: {:?} {:?}", RequestError::BadRequestError, parts);
                return Err(RequestError::BadRequestError);
            }

            let method = parts[0];
            let path = parts[1].to_string();
            let http_version = parts[2].to_string();

            let method = match Method::from(method) {
                Ok(method) => method,
                Err(err) => {
                    eprintln!("Err: {:?}", err);
                    return Err(RequestError::MethodNotAllowedError);
                }
            };

            let r = Self {
                http_version,
                method,
                path,
            };

            Ok(r)
        }
    }

    impl Request {
        pub fn handle(&self) -> Response {
            if self.path == "/" {
                return Response::new(Status::OK);
            }

            if let Some(echo) = self.path.strip_prefix("/echo/") {
                return Response::text(echo);
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
