//! Async Server Sent Event parser and encoder
//!
//! # Examples
//!
//! ```
//! // tbi
//! ```

// ref: https://docs.rs/sse-codec/0.3.0/sse_codec/index.html

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
