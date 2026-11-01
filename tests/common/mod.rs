// Every file in tests/ is a separate crate and not all of them use every
// helper here; without this we'd get spurious dead_code warnings.
#![allow(dead_code)]

//! A tiny HTTP server for tests, with no external dependencies.
//!
//! Replies with predefined (status, body) pairs in order and records the
//! requests it received (request line + headers) so a test can inspect them.
//! Runs on its own thread, so it works for both the async and blocking clients.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

/// What the server saw.
#[derive(Debug)]
pub struct Recorded {
    /// E.g. `GET /api/getdata?year=2025&cc=ru HTTP/1.1`.
    pub request_line: String,
    /// Lower-cased headers: `("user-agent", "...")`.
    pub headers: Vec<(String, String)>,
}

impl Recorded {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }
}

/// How the server answers one connection.
pub enum Reply {
    /// A well-formed response with this status and body.
    Text(u16, &'static str),
    /// Bytes written to the socket as is (for malformed responses).
    Raw(Vec<u8>),
    /// Read the request, then stay silent for a long time (for timeouts).
    Hang,
    /// Write these bytes (e.g. headers and part of the body), then go silent.
    Stall(Vec<u8>),
    /// Read the request, then close the connection without answering.
    Close,
}

pub struct MockServer {
    pub base_url: String,
    requests: mpsc::Receiver<Recorded>,
}

impl MockServer {
    /// Starts a server that will answer `responses.len()` requests.
    pub fn start(responses: Vec<(u16, &'static str)>) -> Self {
        Self::start_with(
            responses
                .into_iter()
                .map(|(status, body)| Reply::Text(status, body))
                .collect(),
        )
    }

    /// Like [`start`](Self::start), but with arbitrary replies.
    pub fn start_with(replies: Vec<Reply>) -> Self {
        // Port 0 means "any free port", so parallel tests don't collide.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}/", listener.local_addr().unwrap());
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            for reply in replies {
                let (mut stream, _) = listener.accept().unwrap();
                let mut reader = BufReader::new(stream.try_clone().unwrap());

                let mut request_line = String::new();
                reader.read_line(&mut request_line).unwrap();
                let mut headers = Vec::new();
                loop {
                    let mut line = String::new();
                    reader.read_line(&mut line).unwrap();
                    let line = line.trim_end();
                    if line.is_empty() {
                        break; // empty line ends the headers (GET has no body)
                    }
                    if let Some((k, v)) = line.split_once(':') {
                        headers.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
                    }
                }
                let _ = tx.send(Recorded {
                    request_line: request_line.trim_end().to_string(),
                    headers,
                });

                // Write errors are ignored: the client may legitimately hang up
                // early (oversized body, timeout).
                match reply {
                    Reply::Text(status, body) => {
                        let response = format!(
                            "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                            body.len()
                        );
                        let _ = stream.write_all(response.as_bytes());
                    }
                    Reply::Raw(bytes) => {
                        let _ = stream.write_all(&bytes);
                    }
                    Reply::Hang => thread::sleep(Duration::from_secs(10)),
                    Reply::Stall(bytes) => {
                        let _ = stream.write_all(&bytes);
                        thread::sleep(Duration::from_secs(10));
                    }
                    Reply::Close => {}
                }
            }
        });

        MockServer {
            base_url,
            requests: rx,
        }
    }

    /// The next received request.
    pub fn next_request(&self) -> Recorded {
        self.requests
            .recv_timeout(Duration::from_secs(5))
            .expect("server did not receive a request")
    }
}

/// A 200 response with a `Content-Length` body of `size` zeros.
pub fn huge_body(size: usize) -> Reply {
    let mut bytes =
        format!("HTTP/1.1 200 OK\r\nContent-Length: {size}\r\nConnection: close\r\n\r\n")
            .into_bytes();
    bytes.resize(bytes.len() + size, b'0');
    Reply::Raw(bytes)
}

/// An address where nothing listens: bind a free port, then release it.
pub fn dead_url() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    format!("http://{}/", listener.local_addr().unwrap())
}
