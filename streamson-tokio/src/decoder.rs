//! Decoders which implement `tokio_util::codec::Decoder`
//! and are able to extract (path, bytes) items for `AsyncRead`
//!

use bytes::{Bytes, BytesMut};
use std::sync::{Arc, Mutex};
use streamson_lib::{error, handler, matcher, strategy};
use tokio_util::codec::Decoder;

/// This struct uses `streamson_lib::matcher` to decode data.
///
/// # Examples
/// ```
/// use std::io;
/// use streamson_lib::{error, matcher};
/// use streamson_tokio::decoder::Extractor;
/// use tokio::{fs, stream::StreamExt};
/// use tokio_util::codec::FramedRead;
///
/// async fn process() -> Result<(), error::General> {
///     let mut file = fs::File::open("/tmp/large.json").await?;
///     let matcher = matcher::Combinator::new(matcher::Simple::new(r#"{"users"}[]"#).unwrap())
///         | matcher::Combinator::new(matcher::Simple::new(r#"{"groups"}[]"#).unwrap());
///     let extractor = Extractor::new(matcher, true);
///     let mut output = FramedRead::new(file, extractor);
///     while let Some(item) = output.next().await {
///         let (path, data) = item?;
///         // Do something with extracted data
///     }
///     Ok(())
/// }
/// ```
pub struct Extractor {
    trigger: strategy::Trigger,
    handler: Arc<Mutex<handler::Buffer>>,
}

impl Extractor {
    /// Creates a new `Extractor`
    ///
    /// # Arguments
    /// * `matcher` - matcher to be used for extractions (see `streamson_lib::matcher`)
    /// * `include_path` - will path be included in output
    pub fn new(matcher: impl matcher::MatchMaker + 'static, include_path: bool) -> Self {
        // TODO limit max length and fail when reached
        let handler = Arc::new(Mutex::new(
            handler::Buffer::new().set_use_path(include_path),
        ));
        let mut trigger = strategy::Trigger::new();
        trigger.add_matcher(Box::new(matcher), &[handler.clone()]);
        Self { trigger, handler }
    }
}

impl Decoder for Extractor {
    type Item = (Option<String>, Bytes);
    type Error = error::General;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        loop {
            {
                // pop if necessary
                let mut handler = self.handler.lock().unwrap();
                if let Some((path, bytes)) = handler.pop() {
                    return Ok(Some((path, Bytes::from(bytes))));
                }
                // handler is unlocked here so it can be used later withing `process` method
            }
            if buf.is_empty() {
                // end has been reached
                return Ok(None);
            }
            let data = buf.split_to(buf.len());
            self.trigger.process(&data[..])?;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Extractor;
    use bytes::Bytes;
    use std::io::Cursor;
    use streamson_lib::matcher;
    use tokio::stream::StreamExt;
    use tokio_util::codec::FramedRead;

    #[tokio::test]
    async fn with_included_path() {
        let cursor =
            Cursor::new(br#"{"users": ["mike","john"], "groups": ["admin", "staff"]}"#.to_vec());
        let matcher = matcher::Combinator::new(matcher::Simple::new(r#"{"users"}[]"#).unwrap())
            | matcher::Combinator::new(matcher::Simple::new(r#"{"groups"}[]"#).unwrap());
        let extractor = Extractor::new(matcher, true);
        let mut output = FramedRead::new(cursor, extractor);

        assert_eq!(
            output.next().await.unwrap().unwrap(),
            (
                Some(r#"{"users"}[0]"#.to_string()),
                Bytes::from_static(br#""mike""#)
            )
        );

        assert_eq!(
            output.next().await.unwrap().unwrap(),
            (
                Some(r#"{"users"}[1]"#.to_string()),
                Bytes::from_static(br#""john""#)
            )
        );

        assert_eq!(
            output.next().await.unwrap().unwrap(),
            (
                Some(r#"{"groups"}[0]"#.to_string()),
                Bytes::from_static(br#""admin""#)
            )
        );

        assert_eq!(
            output.next().await.unwrap().unwrap(),
            (
                Some(r#"{"groups"}[1]"#.to_string()),
                Bytes::from_static(br#""staff""#)
            )
        );

        assert!(output.next().await.is_none());
    }

    #[tokio::test]
    async fn without_included_path() {
        let cursor =
            Cursor::new(br#"{"users": ["mike","john"], "groups": ["admin", "staff"]}"#.to_vec());
        let matcher = matcher::Combinator::new(matcher::Simple::new(r#"{"users"}[]"#).unwrap())
            | matcher::Combinator::new(matcher::Simple::new(r#"{"groups"}[]"#).unwrap());
        let extractor = Extractor::new(matcher, false);
        let mut output = FramedRead::new(cursor, extractor);

        assert_eq!(
            output.next().await.unwrap().unwrap(),
            (None, Bytes::from_static(br#""mike""#))
        );

        assert_eq!(
            output.next().await.unwrap().unwrap(),
            (None, Bytes::from_static(br#""john""#))
        );

        assert_eq!(
            output.next().await.unwrap().unwrap(),
            (None, Bytes::from_static(br#""admin""#))
        );

        assert_eq!(
            output.next().await.unwrap().unwrap(),
            (None, Bytes::from_static(br#""staff""#))
        );

        assert!(output.next().await.is_none());
    }

    #[tokio::test]
    async fn multiple_json_input() {
        let cursor = Cursor::new(
            br#"{"users": ["user1","user2", "user3"]} {"users": ["user4","user5"]}"#.to_vec(),
        );
        let matcher = matcher::Simple::new(r#"{"users"}[]"#).unwrap();
        let extractor = Extractor::new(matcher, true);

        let mut output = FramedRead::new(cursor, extractor);

        assert_eq!(
            output.next().await.unwrap().unwrap(),
            (
                Some(r#"{"users"}[0]"#.to_string()),
                Bytes::from_static(br#""user1""#)
            )
        );
        assert_eq!(
            output.next().await.unwrap().unwrap(),
            (
                Some(r#"{"users"}[1]"#.to_string()),
                Bytes::from_static(br#""user2""#)
            )
        );
        assert_eq!(
            output.next().await.unwrap().unwrap(),
            (
                Some(r#"{"users"}[2]"#.to_string()),
                Bytes::from_static(br#""user3""#)
            )
        );
        assert_eq!(
            output.next().await.unwrap().unwrap(),
            (
                Some(r#"{"users"}[0]"#.to_string()),
                Bytes::from_static(br#""user4""#)
            )
        );
        assert_eq!(
            output.next().await.unwrap().unwrap(),
            (
                Some(r#"{"users"}[1]"#.to_string()),
                Bytes::from_static(br#""user5""#)
            )
        );

        assert!(output.next().await.is_none());
    }
}
