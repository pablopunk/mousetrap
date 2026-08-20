//! Local IPC between the CLI (short-lived, per keystroke) and the resident
//! daemon (owns the overlay, session state, and virtual pointer).
//!
//! Protocol: one JSON request line, one JSON response line, per connection.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const SOCKET_NAME: &str = "mousetrap.sock";

pub fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join(SOCKET_NAME)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", rename_all = "kebab-case")]
pub enum Request {
    Activate,
    Cancel,
    KeyDown { key: String },
    KeyUp { key: String },
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    /// Exit code for the CLI: 0 = ok, 1 = error, 3 = final commit (used by
    /// compositor binds to reset the input submap).
    pub exit_code: i32,
    pub message: String,
}

impl Response {
    pub fn ok(message: impl Into<String>) -> Self {
        Self { ok: true, exit_code: 0, message: message.into() }
    }
    pub fn err(message: impl Into<String>) -> Self {
        Self { ok: false, exit_code: 1, message: message.into() }
    }
    pub fn with_code(mut self, code: i32) -> Self {
        self.exit_code = code;
        self
    }
}

/// Client side: send one request, read one response.
pub fn send(request: &Request) -> std::io::Result<Response> {
    let mut stream = UnixStream::connect(socket_path())?;
    let line = serde_json::to_string(request).expect("serialize request");
    stream.write_all(line.as_bytes())?;
    stream.write_all(b"\n")?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    Ok(serde_json::from_str(&response).unwrap_or_else(|_| Response::err("bad daemon response")))
}
