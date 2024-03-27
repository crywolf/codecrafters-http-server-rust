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

pub struct Response {
    status: BytesMut,
}

impl Response {
    pub fn new(status: Status) -> Self {
        let status_line = match status {
            Status::OK => Self::STATUS_200_OK,
            Status::BadRequest => Self::STATUS_400_BAD_REQUEST,
            Status::NotFound => Self::STATUS_404_NOT_FOUND,
            Status::MethodNotAllowed => Self::STATUS_405_METHOD_NOT_ALLOWED,
        };

        let mut status = BytesMut::with_capacity(64);
        status.extend_from_slice(b"HTTP/1.1 ");
        status.extend_from_slice(status_line.as_bytes());
        status.extend_from_slice(b"\r\n\r\n");

        Self { status }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.status
    }

    const STATUS_200_OK: &'static str = "200 OK";
    const STATUS_400_BAD_REQUEST: &'static str = "400 Bad Request";
    const STATUS_404_NOT_FOUND: &'static str = "404 Not Found";
    const STATUS_405_METHOD_NOT_ALLOWED: &'static str = "405 Method Not Allowed";
}
