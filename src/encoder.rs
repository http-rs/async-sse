use async_std::io::Write as AsyncWrite;
use std::io;
use std::time::Duration;

use crate::Message;

/// An SSE protocol encoder.
#[derive(Debug)]
pub struct Encoder;

/// Encode a new SSE connection.
pub fn encode(_s: impl AsyncWrite) -> Encoder {
    todo!();
}

impl Encoder {
    /// Send a new message over SSE.
    pub fn send(&self, _msg: Message) -> io::Result<()> {
        todo!();
    }
    /// Send a new "retry" message over SSE.
    pub fn send_retry(&self, _dur: Duration) -> io::Result<()> {
        todo!();
    }
}
