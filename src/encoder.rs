use async_std::io::prelude::*;
use async_std::io::Write as AsyncWrite;
use std::io;
use std::time::Duration;

/// An SSE protocol encoder.
#[derive(Debug)]
pub struct Encoder<W> {
    writer: W,
}

/// Encode a new SSE connection.
pub fn encode<W: AsyncWrite + Unpin>(writer: W) -> Encoder<W> {
    Encoder { writer }
}

impl<W> Encoder<W> {
    /// Access the inner writer from the Encoder.
    pub fn into_writer(self) -> W {
        self.writer
    }
}

impl<W: AsyncWrite + Unpin> Encoder<W> {
    /// Send a new message over SSE.
    pub async fn send(&mut self, name: &str, data: &[u8], id: Option<&str>) -> io::Result<()> {
        // Write the event name
        self.writer.write_all(b"event:").await?;
        self.writer.write_all(name.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;

        // Write the id
        if let Some(id) = id {
            self.writer.write_all(b"id:").await?;
            self.writer.write_all(id.as_bytes()).await?;
            self.writer.write_all(b"\n").await?;
        }

        // Write the section
        self.writer.write_all(b"data:").await?;
        self.writer.write_all(data).await?;
        self.writer.write_all(b"\n").await?;

        // Finalize the message
        self.writer.write_all(b"\n").await?;

        Ok(())
    }

    /// Send a new "retry" message over SSE.
    pub async fn send_retry(&mut self, dur: Duration, id: Option<&str>) -> io::Result<()> {
        // Write the id
        if let Some(id) = id {
            self.writer.write_all(b"id:").await?;
            self.writer.write_all(id.as_bytes()).await?;
            self.writer.write_all(b"\n").await?;
        }

        // Write the section
        self.writer.write_all(b"retry:").await?;
        self.writer
            .write_all(&format!("{}", dur.as_secs_f64() as u64).as_bytes())
            .await?;
        self.writer.write_all(b"\n").await?;

        // Finalize the message
        self.writer.write_all(b"\n").await?;

        Ok(())
    }
}
