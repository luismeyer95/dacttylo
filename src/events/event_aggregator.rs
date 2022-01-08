use futures::{stream::SelectAll, Stream, StreamExt};
use std::{
    pin::Pin,
    task::{Context, Poll},
};

#[macro_export]
macro_rules! aggregate {
    ([$($item:expr),+] as $t:ty) => {{
        let mut aggr = EventAggregator::<$t>::new();
        aggr$(.push(Box::pin($item)))+;
        aggr
    }};
}
pub struct EventAggregator<T> {
    event_stream: SelectAll<Box<dyn Stream<Item = T> + Unpin>>,
}

impl<T> EventAggregator<T>
where
    T: 'static,
{
    pub fn new() -> Self {
        Self {
            event_stream: Default::default(),
        }
    }

    pub fn push<U>(&mut self, stream: impl Stream<Item = U> + Unpin + 'static) -> &mut Self
    where
        U: Into<T> + 'static,
    {
        self.event_stream
            .push(Box::new(stream.map(Into::<T>::into)));
        self
    }
}

impl<T> Stream for EventAggregator<T> {
    type Item = T;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.event_stream).poll_next(cx)
    }
}

#[cfg(test)]
mod tests {

    use std::time::Duration;

    use super::*;
    use async_stream::stream;
    use tokio_stream::iter;

    #[tokio::test]
    async fn basic() {
        let mut events = EventAggregator::<u64>::new();
        events
            .push(iter([1u8]))
            .push(iter([2u16]))
            .push(iter([3u32]));

        assert_eq!(events.collect::<Vec<_>>().await, &[1u64, 2u64, 3u64]);
    }

    #[tokio::test]
    async fn advanced() {
        let sleep_ms = |ms| tokio::time::sleep(Duration::from_millis(ms));

        #[derive(Eq, PartialEq, Debug)]
        struct Foo(u32);

        #[derive(Eq, PartialEq, Debug)]
        struct Bar(u32);

        impl From<Foo> for Bar {
            fn from(foo: Foo) -> Bar {
                Bar(foo.0 * 10)
            }
        }

        let events = aggregate!([
            stream! {
                for i in (3..6) {
                    sleep_ms(5).await;
                    yield Foo(i);
                }
            },
            iter([Foo(1)]),
            iter([Foo(2)])
        ] as Bar);

        assert_eq!(
            events.collect::<Vec<_>>().await,
            &[Bar(10), Bar(20), Bar(30), Bar(40), Bar(50)]
        );
    }
}
