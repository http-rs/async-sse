use std::io;
use std::mem;
use std::pin::Pin;
use std::str;

use pin_project_lite::pin_project;

use futures_lite::prelude::*;
use futures_lite::ready;
use std::task::{Context, Poll};

pin_project! {
    /// A stream of lines in a byte stream.
    ///
    /// This stream is created by the [`lines`] method on types that implement [`BufRead`].
    ///
    /// This type is an async version of [`std::io::Lines`].
    ///
    /// [`lines`]: trait.BufRead.html#method.lines
    /// [`BufRead`]: trait.BufRead.html
    /// [`std::io::Lines`]: https://doc.rust-lang.org/std/io/struct.Lines.html
    #[derive(Debug)]
    pub(crate) struct Lines<R> {
        #[pin]
        pub(crate) reader: R,
        pub(crate) buf: String,
        pub(crate) bytes: Vec<u8>,
        pub(crate) read: usize,
    }
}

impl<R> Lines<R> {
    pub(crate) fn new(reader: R) -> Lines<R>
    where
        R: AsyncBufRead + Unpin + Sized,
    {
        Lines {
            reader,
            buf: String::new(),
            bytes: Vec::new(),
            read: 0,
        }
    }
}

impl<R: AsyncBufRead> Stream for Lines<R> {
    type Item = io::Result<String>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = self.project();
        let n = ready!(read_line_internal(
            this.reader,
            cx,
            this.buf,
            this.bytes,
            this.read
        ))?;
        if n == 0 && this.buf.is_empty() {
            return Poll::Ready(None);
        }
        if this.buf.ends_with('\n') {
            this.buf.pop();
        }
        if this.buf.ends_with('\r') {
            this.buf.pop();
        }
        Poll::Ready(Some(Ok(mem::replace(this.buf, String::new()))))
    }
}

fn read_line_internal<R: AsyncBufRead + ?Sized>(
    reader: Pin<&mut R>,
    cx: &mut Context<'_>,
    buf: &mut String,
    bytes: &mut Vec<u8>,
    read: &mut usize,
) -> Poll<io::Result<usize>> {
    let ret = ready!(read_until_internal(reader, cx, bytes, read));
    if str::from_utf8(&bytes).is_err() {
        Poll::Ready(ret.and_then(|_| {
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "stream did not contain valid UTF-8",
            ))
        }))
    } else {
        debug_assert!(buf.is_empty());
        debug_assert_eq!(*read, 0);
        // Safety: `bytes` is a valid UTF-8 because `str::from_utf8` returned `Ok`.
        mem::swap(unsafe { buf.as_mut_vec() }, bytes);
        Poll::Ready(ret)
    }
}

fn read_until_internal<R: AsyncBufRead + ?Sized>(
    mut reader: Pin<&mut R>,
    cx: &mut Context<'_>,
    buf: &mut Vec<u8>,
    read: &mut usize,
) -> Poll<io::Result<usize>> {
    loop {
        let (done, used) = {
            let available = ready!(reader.as_mut().poll_fill_buf(cx))?;
            if let Some(i) = memchr::memchr2(b'\r', b'\n', available) {
                buf.extend_from_slice(&available[..=i]);
                // Remove any tailing \r or \n characters.
                match available.get(i + 1) {
                    Some(c) if available[i] == b'\r' && *c == b'\n' => (true, i + 2),
                    _ => (true, i + 1),
                }
            } else {
                buf.extend_from_slice(available);
                (false, available.len())
            }
        };
        reader.as_mut().consume(used);
        *read += used;
        if done || used == 0 {
            return Poll::Ready(Ok(mem::replace(read, 0)));
        }
    }
}
