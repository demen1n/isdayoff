// Каждый файл в tests/ — отдельный крейт, и не каждый использует все
// хелперы отсюда. Без этого получим ложные предупреждения dead_code.
#![allow(dead_code)]

//! Крошечный HTTP-сервер для тестов — без внешних зависимостей.
//!
//! Отвечает заранее заданными (статус, тело) по очереди и запоминает, какие
//! запросы к нему пришли (строка запроса + заголовки), чтобы тест мог их
//! проверить. Работает в отдельном потоке, поэтому подходит и для async-,
//! и для blocking-клиента.

use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread;

/// Что увидел сервер.
#[derive(Debug)]
pub struct Recorded {
    /// Например `GET /api/getdata?year=2025&cc=ru HTTP/1.1`.
    pub request_line: String,
    /// Заголовки в нижнем регистре: `("user-agent", "...")`.
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
    /// Запускает сервер, который ответит на `responses.len()` запросов.
    pub fn start(responses: Vec<(u16, &'static str)>) -> Self {
        // Порт 0 — «дай любой свободный»: тесты не мешают друг другу.
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
                        break; // пустая строка — конец заголовков (у GET нет тела)
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

    /// Следующий принятый запрос.
    pub fn next_request(&self) -> Recorded {
        self.requests
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("server did not receive a request")
    }
}
