use async_sse::{decode, encode, Event};
use async_std::io::Cursor;
use async_std::prelude::*;
use std::time::Duration;

/// Assert a Message.
fn assert_message(event: &Event, name: &str, data: &str, id: Option<&'static str>) {
    assert!(event.is_message());
    if let Event::Message(msg) = event {
        assert_eq!(msg.id(), &id.map(|s| s.to_owned()));
        assert_eq!(msg.name(), name);
        assert_eq!(
            String::from_utf8(msg.data().to_owned()).unwrap(),
            String::from_utf8(data.as_bytes().to_owned()).unwrap()
        );
    }
}

/// Assert a Message.
fn assert_retry(event: &Event, dur: u64) {
    assert!(event.is_retry());
    let expected = Duration::from_secs_f64(dur as f64);
    if let Event::Retry(dur) = event {
        assert_eq!(dur, &expected);
    }
}
#[async_std::test]
async fn encode_message() -> http_types::Result<()> {
    let buf = Cursor::new(vec![]);
    let mut encoder = encode(buf);
    encoder.send("cat", b"chashu", None).await?;
    let mut buf = encoder.into_writer();
    buf.set_position(0);

    let mut reader = decode(buf);
    let event = reader.next().await.unwrap()?;
    assert_message(&event, "cat", "chashu", None);
    Ok(())
}

#[async_std::test]
async fn encode_message_with_id() -> http_types::Result<()> {
    let buf = Cursor::new(vec![]);
    let mut encoder = encode(buf);
    encoder.send("cat", b"chashu", Some("0")).await?;
    let mut buf = encoder.into_writer();
    buf.set_position(0);

    let mut reader = decode(buf);
    let event = reader.next().await.unwrap()?;
    assert_message(&event, "cat", "chashu", Some("0"));
    Ok(())
}

#[async_std::test]
async fn encode_retry() -> http_types::Result<()> {
    let buf = Cursor::new(vec![]);
    let mut encoder = encode(buf);
    encoder.send_retry(Duration::from_secs(12), None).await?;
    let mut buf = encoder.into_writer();
    buf.set_position(0);

    let mut reader = decode(buf);
    let event = reader.next().await.unwrap()?;
    assert_retry(&event, 12);
    Ok(())
}
