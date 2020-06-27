use async_std::io::Read as AsyncRead;
use async_std::prelude::*;
use async_std::task::{ready, Context, Poll};

use std::io;
use std::pin::Pin;
use std::time::Duration;

pin_project_lite::pin_project! {
    /// An SSE protocol encoder.
    #[derive(Debug)]
    pub struct Encoder {
        buf: Option<Vec<u8>>,
        #[pin]
        receiver: async_channel::Receiver<Vec<u8>>,
        cursor: usize,
    }
}

impl AsyncRead for Encoder {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        // Request a new buffer if we don't have one yet.
        if let None = self.buf {
            self.buf = match ready!(Pin::new(&mut self.receiver).poll_next(cx)) {
                Some(buf) => {
                    log::trace!("> Received a new buffer with len {}", buf.len());
                    Some(buf)
                }
                None => {
                    log::trace!("> Encoder done reading");
                    return Poll::Ready(Ok(0));
                }
            };
        };

        // Write the current buffer to completion.
        let local_buf = self.buf.as_mut().unwrap();
        let local_len = local_buf.len();
        let max = buf.len().min(local_buf.len());
        buf[..max].clone_from_slice(&local_buf[..max]);

        self.cursor += max;

        // Reset values if we're done reading.
        if self.cursor == local_len {
            self.buf = None;
            self.cursor = 0;
        };

        // Return bytes read.
        Poll::Ready(Ok(max))
    }
}

// impl AsyncBufRead for Encoder {
//     fn poll_fill_buf(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<&[u8]>> {
//         match ready!(self.project().receiver.poll_next(cx)) {
//             Some(buf) => match &self.buf {
//                 None => self.project().buf = &mut Some(buf),
//                 Some(local_buf) => local_buf.extend(buf),
//             },
//             None => {
//                 if let None = self.buf {
//                     self.project().buf = &mut Some(vec![]);
//                 };
//             }
//         };
//         Poll::Ready(Ok(self.buf.as_ref().unwrap()))
//     }

//     fn consume(self: Pin<&mut Self>, amt: usize) {
//         Pin::new(self).cursor += amt;
//     }
// }

/// The sending side of the encoder.
#[derive(Debug)]
pub struct Sender(async_channel::Sender<Vec<u8>>);

/// Create a new SSE encoder.
pub fn encode() -> (Sender, Encoder) {
    let (sender, receiver) = async_channel::bounded(1);
    let encoder = Encoder {
        receiver,
        buf: None,
        cursor: 0,
    };
    (Sender(sender), encoder)
}

impl Sender {
    async fn inner_send(&self, bytes: impl Into<Vec<u8>>) -> io::Result<()> {
        self.0
            .send(bytes.into())
            .await
            .map_err(|_| io::Error::new(io::ErrorKind::ConnectionAborted, "sse disconnected"))
    }

    /// Send a new message over SSE.
    pub async fn send(&self, name: &str, data: &str, id: Option<&str>) -> io::Result<()> {
        // Write the event name
        let msg = format!("event:{}\n", name);
        self.inner_send(msg).await?;

        // Write the id
        if let Some(id) = id {
            self.inner_send(format!("id:{}\n", id)).await?;
        }

        // Write the data section, and end.
        let msg = format!("data:{}\n\n", data);
        self.inner_send(msg).await?;

        Ok(())
    }

    /// Send a new "retry" message over SSE.
    pub async fn send_retry(&self, dur: Duration, id: Option<&str>) -> io::Result<()> {
        // Write the id
        if let Some(id) = id {
            self.inner_send(format!("id:{}\n", id)).await?;
        }

        // Write the retry section, and end.
        let dur = dur.as_secs_f64() as u64;
        let msg = format!("retry:{}\n\n", dur);
        self.inner_send(msg).await?;
        Ok(())
    }
}

impl Clone for Sender {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}
