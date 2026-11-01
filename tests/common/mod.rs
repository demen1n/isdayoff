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

pub struct MockServer {
    pub base_url: String,
    requests: mpsc::Receiver<Recorded>,
}

impl MockServer {
    /// Starts a server that will answer `responses.len()` requests.
    pub fn start(responses: Vec<(u16, &'static str)>) -> Self {
        // Port 0 means "any free port", so parallel tests don't collide.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base_url = format!("http://{}/", listener.local_addr().unwrap());
        let (tx, rx) = mpsc::channel();

        thread::spawn(move || {
            for (status, body) in responses {
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

                let response = format!(
                    "HTTP/1.1 {status} X\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                stream.write_all(response.as_bytes()).unwrap();
                let _ = tx.send(Recorded {
                    request_line: request_line.trim_end().to_string(),
                    headers,
                });
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
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("server did not receive a request")
    }
}
