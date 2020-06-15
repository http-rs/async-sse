use async_std::sync;
use std::io;
use std::time::Duration;

use async_std::io::Read as AsyncRead;
use async_std::prelude::*;
use async_std::task::{ready, Context, Poll};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use pin_project::{pin_project, pinned_drop};

#[pin_project(PinnedDrop)]
/// An SSE protocol encoder.
#[derive(Debug)]
pub struct Encoder {
    buf: Option<Vec<u8>>,
    #[pin]
    receiver: sync::Receiver<Vec<u8>>,
    cursor: usize,
    disconnected: Arc<AtomicBool>,
}

#[pinned_drop]
impl PinnedDrop for Encoder {
    fn drop(self: Pin<&mut Self>) {
        self.disconnected.store(true, Ordering::Relaxed);
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
#[derive(Debug, Clone)]
pub struct Sender {
    sender: sync::Sender<Vec<u8>>,
    disconnected: Arc<std::sync::atomic::AtomicBool>,
}

/// Create a new SSE encoder.
pub fn encode() -> (Sender, Encoder) {
    let (sender, receiver) = sync::channel(1);
    let disconnected = Arc::new(AtomicBool::new(false));

    let encoder = Encoder {
        receiver,
        buf: None,
        cursor: 0,
        disconnected: disconnected.clone(),
    };

    let sender = Sender {
        sender,
        disconnected,
    };

    (sender, encoder)
}

/// An error that represents that the [Encoder] has been dropped.
#[derive(Debug, Eq, PartialEq)]
pub struct DisconnectedError;
impl std::error::Error for DisconnectedError {}
impl std::fmt::Display for DisconnectedError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Disconnected")
    }
}

#[must_use]
impl Sender {
    /// Send a new message over SSE.
    pub async fn send(
        &self,
        name: &str,
        data: &str,
        id: Option<&str>,
    ) -> Result<(), DisconnectedError> {
        if self.disconnected.load(Ordering::Relaxed) {
            return Err(DisconnectedError);
        }

        // Write the event name
        let msg = format!("event:{}\n", name);
        self.sender.send(msg.into_bytes()).await;

        // Write the id
        if let Some(id) = id {
            self.sender.send(format!("id:{}\n", id).into_bytes()).await;
        }

        // Write the data section, and end.
        let msg = format!("data:{}\n\n", data);
        self.sender.send(msg.into_bytes()).await;
        Ok(())
    }

    /// Send a new "retry" message over SSE.
    pub async fn send_retry(&self, dur: Duration, id: Option<&str>) {
        // Write the id
        if let Some(id) = id {
            self.sender.send(format!("id:{}\n", id).into_bytes()).await;
        }

        // Write the retry section, and end.
        let dur = dur.as_secs_f64() as u64;
        let msg = format!("retry:{}\n\n", dur);
        self.sender.send(msg.into_bytes()).await;
    }
}
