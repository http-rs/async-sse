//! Async Server Sent Event parser and encoder.
//!
//! # Example
//!
//! ```
//! use async_sse::{decode, encode, Event};
//! use async_std::io::Cursor;
//! use async_std::prelude::*;
//!
//! #[async_std::main]
//! async fn main() -> http_types::Result<()> {
//!     let buf = Cursor::new(vec![]);
//!
//!     // Encode messages to an AsyncWrite.
//!     let mut encoder = encode(buf);
//!     encoder.send("cat", b"chashu", None).await?;
//!
//!     let mut buf = encoder.into_writer();
//!     buf.set_position(0);
//!
//!     // Decode messages from an AsyncRead.
//!     let mut reader = decode(buf);
//!     let event = reader.next().await.unwrap()?;
//!     // Match and handle the event
//!
//!     # let _ = event;
//!     Ok(())
//! }
//! ```
//!
//! # References
//!
//! - [SSE Spec](https://html.spec.whatwg.org/multipage/server-sent-events.html#concept-event-stream-last-event-id)
//! - [EventSource web platform tests](https://github.com/web-platform-tests/wpt/tree/master/eventsource)

#![forbid(rust_2018_idioms)]
#![deny(missing_debug_implementations, nonstandard_style)]
#![warn(missing_docs, missing_doc_code_examples, unreachable_pub)]

mod decoder;
mod encoder;
mod event;
mod lines;
mod message;

pub use decoder::{decode, Decoder};
pub use encoder::{encode, Encoder};
pub use event::Event;
pub use message::Message;

pub(crate) use lines::Lines;
