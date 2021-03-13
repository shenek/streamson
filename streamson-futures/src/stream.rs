//! Integration of futures::stream with streamson
//!

use std::{
    marker::Unpin,
    pin::Pin,
    sync::{Arc, Mutex},
};

use bytes::Bytes;
use futures::{
    task::{Context, Poll},
    Stream,
};
use streamson_lib::{
    error::General as StreamsonError,
    handler, matcher,
    strategy::{self, Strategy},
};

/// This struct is used to wrap Bytes input stream to
/// (Path, Bytes) - the matched path and matched bytes in json stream
/// # Examples
/// ```
/// # futures::executor::block_on(async {
///
/// use bytes::Bytes;
/// use futures::stream::{self, StreamExt};
/// use streamson_lib::matcher;
/// use streamson_futures::stream::BufferStream;
///
/// let stream = stream::iter(
///     vec![r#"{"users": ["#, r#"{"name": "carl", "id": 1}"#, r#"]}"#]
///         .drain(..)
///         .map(Bytes::from)
///         .collect::<Vec<Bytes>>()
/// );
/// let matcher = matcher::Simple::new(r#"{"users"}[]{"name"}"#).unwrap();
/// let wrapped_stream = BufferStream::new(stream, Box::new(matcher));
/// # });
/// ```
pub struct BufferStream<I>
where
    I: Stream<Item = Bytes> + Unpin,
{
    input: I,
    trigger: Arc<Mutex<strategy::Trigger>>,
    buffer: Arc<Mutex<handler::Buffer>>,
}

impl<I> BufferStream<I>
where
    I: Stream<Item = Bytes> + Unpin,
{
    /// Wraps stream to extracts json paths defined by the matcher
    ///
    /// # Arguments
    /// * `input` - input stram to be matched
    /// * `matcher` - matcher which will be used for the extraction
    pub fn new(input: I, matcher: Box<dyn matcher::MatchMaker>) -> Self {
        let trigger = Arc::new(Mutex::new(strategy::Trigger::new()));
        let buffer = Arc::new(Mutex::new(handler::Buffer::new().set_use_path(true)));
        trigger.lock().unwrap().add_matcher(matcher, buffer.clone());
        Self {
            input,
            trigger,
            buffer,
        }
    }
}

impl<I> Stream for BufferStream<I>
where
    I: Stream<Item = Bytes> + Unpin,
{
    type Item = Result<(String, Bytes), StreamsonError>;
    fn poll_next(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<Option<Self::Item>> {
        loop {
            // Check whether there are data in the buffer
            if let Some((path, data)) = self.buffer.lock().unwrap().pop() {
                return Poll::Ready(Some(Ok((path.unwrap(), Bytes::from(data)))));
            }
            // Try to process new data with the trigger
            match Pin::new(&mut self.input).poll_next(ctx) {
                Poll::Ready(Some(bytes)) => {
                    self.trigger.lock().unwrap().process(&bytes)?;
                }
                Poll::Ready(None) => {
                    return Poll::Ready(None);
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

#[cfg(test)]
mod test {
    use bytes::Bytes;
    use futures::stream::{self, StreamExt};
    use streamson_lib::matcher;

    use super::BufferStream;

    #[tokio::test]
    async fn test_basic() {
        let stream = stream::iter(
            vec![
                r#"{"users": ["#,
                r#"{"name": "carl",
                "id": 1}"#,
                r#"]}"#,
            ]
            .drain(..)
            .map(Bytes::from)
            .collect::<Vec<Bytes>>(),
        );
        let matcher = matcher::Simple::new(r#"{"users"}[]{"name"}"#).unwrap();
        let wrapped_stream = BufferStream::new(stream, Box::new(matcher));
        let mut collected = wrapped_stream
            .collect::<Vec<Result<(String, Bytes), _>>>()
            .await;

        assert_eq!(
            vec![(
                String::from(r#"{"users"}[0]{"name"}"#),
                Bytes::from(r#""carl""#)
            )],
            collected
                .drain(..)
                .map(|e| e.unwrap())
                .collect::<Vec<(String, Bytes)>>()
        );
    }

    #[tokio::test]
    async fn test_error() {
        let stream = stream::iter(
            vec![
                r#"{"users": ["#,
                r#"{"name": "carl",
                "id": 1}"#,
                r#"}]}"#,
            ]
            .drain(..)
            .map(Bytes::from)
            .collect::<Vec<Bytes>>(),
        );
        let matcher = matcher::Simple::new(r#"{"users"}[]{"name"}"#).unwrap();
        let wrapped_stream = BufferStream::new(stream, Box::new(matcher));
        let collected = wrapped_stream
            .collect::<Vec<Result<(String, Bytes), _>>>()
            .await;
        assert!(collected[0].is_ok());
        assert!(collected[1].is_err());
    }
}
