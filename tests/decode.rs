use async_sse::{decode, Message};
use async_std::io::Cursor;
use async_std::prelude::*;

/// Assert a Message.
fn assert_msg(msg: &Message, name: &str, data: &str, id: Option<&'static str>) {
    assert_eq!(msg.id(), &id.map(|s| s.to_owned()));
    assert_eq!(msg.name(), name);
    assert_eq!(String::from_utf8(msg.data().to_owned()).unwrap(), String::from_utf8(data.as_bytes().to_owned()).unwrap());
    // assert_eq!(msg.data(), data.as_bytes());
}

#[async_std::test]
async fn simple_event() -> http_types::Result<()> {
    let input = Cursor::new("event: add\ndata: test\ndata: test2\n\n");
    let mut reader = decode(input);
    let msg = reader.next().await.unwrap()?;
    assert_eq!(msg.id(), &None);
    assert_eq!(msg.name(), "add");
    assert_eq!(msg.data(), b"test\ntest2");
    Ok(())
}

#[async_std::test]
async fn decode_stream_when_fed_by_line() -> http_types::Result<()> {
    let reader = decode(Cursor::new(":ok\nevent:message\nid:id1\ndata:data1\n\n"));
    let res = reader.map(|i| i.unwrap()).collect::<Vec<_>>().await;
    assert_eq!(res.len(), 1);
    assert_msg(&res.get(0).unwrap(), "message", "data1", Some("id1"));
    Ok(())
}

#[async_std::test]
async fn maintain_id_state() -> http_types::Result<()> {
    let reader = decode(Cursor::new("id:1\ndata:messageone\n\ndata:messagetwo\n\n"));
    let mut res = reader.map(|i| i.unwrap()).collect::<Vec<_>>().await;
    assert_eq!(res.len(), 2);
    assert_msg(&res.remove(0), "message", "messageone", Some("1"));
    assert_msg(&res.remove(0), "message", "messagetwo", Some("1"));
    Ok(())
}

/// https://github.com/web-platform-tests/wpt/blob/master/eventsource/event-data.html
#[async_std::test]
async fn event_data() -> http_types::Result<()> {
    femme::start(log::LevelFilter::Trace)?;
    let input = concat!(
        "data:msg\n",
        "data:msg\n",
        "\n",
        ":\n",
        "falsefield:msg\n",
        "\n",
        "falsefield:msg\n",
        "Data:data\n",
        "\n",
        "data\n",
        "\n",
        "data:end\n",
        "\n",
    );
    let mut reader = decode(Cursor::new(input));
    assert_msg(&reader.next().await.unwrap()?, "message", "msg\nmsg", None);
    assert_msg(&reader.next().await.unwrap()?, "message", "end", None);
    assert!(reader.next().await.is_none());
    Ok(())
}

/// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-bom.htm
/// The byte order marker should only be stripped at the very start.
#[async_std::test]
async fn bom() -> http_types::Result<()> {
    let mut input = vec![];
    input.extend(b"\xEF\xBB\xBF");
    input.extend(b"data:1\n");
    input.extend(b"\n");
    input.extend(b"\xEF\xBB\xBF");
    input.extend(b"data:2\n");
    input.extend(b"\n");
    input.extend(b"data:3\n");
    input.extend(b"\n");
    let mut reader = decode(Cursor::new(input));
    assert_msg(&reader.next().await.unwrap()?, "message", "1", None);
    assert_msg(&reader.next().await.unwrap()?, "message", "3", None);
    assert!(reader.next().await.is_none());
    Ok(())
}

/// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-bom-2.htm
/// Only _one_ byte order marker should be stripped. This has two, which means one will remain
/// in the first line, therefore making the first `data:1` invalid.
#[async_std::test]
async fn bom2() -> http_types::Result<()> {
    let mut input = vec![];
    input.extend(b"\xEF\xBB\xBF");
    input.extend(b"\xEF\xBB\xBF");
    input.extend(b"data:1\n");
    input.extend(b"\n");
    input.extend(b"data:2\n");
    input.extend(b"\n");
    input.extend(b"data:3\n");
    input.extend(b"\n");
    let mut reader = decode(Cursor::new(input));
    assert_msg(&reader.next().await.unwrap()?, "message", "2", None);
    assert_msg(&reader.next().await.unwrap()?, "message", "3", None);
    assert!(reader.next().await.is_none());
    Ok(())
}

/// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-comments.htm
#[async_std::test]
async fn comments() -> http_types::Result<()> {
    let longstring = "x".repeat(2049);
    let mut input = concat!("data:1\r", ":\0\n", ":\r\n", "data:2\n", ":").to_string();
    input.push_str(&longstring);
    input.push_str("\r");
    input.push_str("data:3\n");
    input.push_str(":data:fail\r");
    input.push_str(":");
    input.push_str(&longstring);
    input.push_str("\n");
    input.push_str("data:4\n\n");
    let mut reader = decode(Cursor::new(input));
    assert_msg(&reader.next().await.unwrap()?, "message", "1\n2\n3\n4", None);
    assert!(reader.next().await.is_none());
    Ok(())
}

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-data-before-final-empty-line.htm
// #[test]
// fn data_before_final_empty_line() {
//     let input = "retry:1000\ndata:test1\n\nid:test\ndata:test2";
//     let mut messages = decode(input.as_bytes());
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Retry { retry: 1000 })
//     );
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "message".into(),
//             data: "test1".into()
//         })
//     );
//     assert!(dbg!(messages.next()).is_none());
// }

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-field-data.htm
// #[test]
// fn field_data() {
//     let input = "data:\n\ndata\ndata\n\ndata:test\n\n";
//     let mut messages = decode(input.as_bytes());
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "message".into(),
//             data: "".into()
//         })
//     );
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "message".into(),
//             data: "\n".into()
//         })
//     );
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "message".into(),
//             data: "test".into()
//         })
//     );
//     assert!(messages.next().is_none());
// }

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-field-event-empty.htm
// #[test]
// fn field_event_empty() {
//     let input = "event: \ndata:data\n\n";
//     let mut messages = decode(input.as_bytes());
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "".into(),
//             data: "data".into()
//         })
//     );
//     assert!(messages.next().is_none());
// }

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-field-event.htm
// #[test]
// fn field_event() {
//     let input = "event:test\ndata:x\n\ndata:x\n\n";
//     let mut messages = decode(input.as_bytes());
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "test".into(),
//             data: "x".into()
//         })
//     );
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "message".into(),
//             data: "x".into()
//         })
//     );
//     assert!(messages.next().is_none());
// }

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-field-id.htm
// #[test]
// #[ignore]
// fn field_id() {
//     unimplemented!()
// }

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-field-id-2.htm
// #[test]
// #[ignore]
// fn field_id_2() {
//     unimplemented!()
// }

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-field-parsing.htm
// #[test]
// fn field_parsing() {
//     let input = "data:\0\ndata:  2\rData:1\ndata\0:2\ndata:1\r\0data:4\nda-ta:3\rdata_5\ndata:3\rdata:\r\n data:32\ndata:4\n\n";
//     let mut messages = decode(input.as_bytes());
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "message".into(),
//             data: "\0\n 2\n1\n3\n\n4".into()
//         })
//     );
//     assert!(messages.next().is_none());
// }

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-field-retry-bogus.htm
// #[test]
// fn field_retry_bogus() {
//     let input = "retry:3000\nretry:1000x\ndata:x\n\n";
//     let mut messages = decode(input.as_bytes());
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Retry { retry: 3000 })
//     );
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "message".into(),
//             data: "x".into()
//         })
//     );
//     assert!(messages.next().is_none());
// }

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-field-retry-empty.htm
// #[test]
// fn field_retry_empty() {
//     let input = "retry\ndata:test\n\n";
//     let mut messages = decode(input.as_bytes());
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "message".into(),
//             data: "test".into()
//         })
//     );
//     assert!(messages.next().is_none());
// }

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-field-retry.htm
// #[test]
// fn field_retry() {
//     let input = "retry:03000\ndata:x\n\n";
//     let mut messages = decode(input.as_bytes());
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Retry { retry: 3000 })
//     );
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "message".into(),
//             data: "x".into()
//         })
//     );
//     assert!(messages.next().is_none());
// }

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-field-unknown.htm
// #[test]
// fn field_unknown() {
//     let input =
//         "data:test\n data\ndata\nfoobar:xxx\njustsometext\n:thisisacommentyay\ndata:test\n\n";
//     let mut messages = decode(input.as_bytes());
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "message".into(),
//             data: "test\n\ntest".into()
//         })
//     );
//     assert!(messages.next().is_none());
// }

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-leading-space.htm
// #[test]
// fn leading_space() {
//     let input = "data:\ttest\rdata: \ndata:test\n\n";
//     let mut messages = decode(input.as_bytes());
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "message".into(),
//             data: "\ttest\n\ntest".into()
//         })
//     );
//     assert!(messages.next().is_none());
// }

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-newlines.htm
// #[test]
// fn newlines() {
//     let input = "data:test\r\ndata\ndata:test\r\n\r";
//     let mut messages = decode(input.as_bytes());
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "message".into(),
//             data: "test\n\ntest".into()
//         })
//     );
//     assert!(messages.next().is_none());
// }

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-null-character.html
// #[test]
// fn null_character() {
//     let input = "data:\0\n\n\n\n";
//     let mut messages = decode(input.as_bytes());
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "message".into(),
//             data: "\0".into()
//         })
//     );
//     assert!(messages.next().is_none());
// }

// /// https://github.com/web-platform-tests/wpt/blob/master/eventsource/format-utf-8.htm
// #[test]
// fn utf_8() {
//     let input = b"data:ok\xE2\x80\xA6\n\n";
//     let mut messages = decode(input);
//     assert_eq!(
//         messages.next().map(Result::unwrap),
//         Some(Event::Message {
//             id: None,
//             event: "message".into(),
//             data: "ok…".into()
//         })
//     );
//     assert!(messages.next().is_none());
// }
