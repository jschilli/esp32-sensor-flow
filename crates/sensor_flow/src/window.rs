use core::fmt;
use core::pin::Pin;
use futures_core::ready;
use futures_core::task::{Context, Poll};
#[cfg(feature = "sink")]
use futures_sink::Sink;
use futures_util::stream::{FusedStream, Stream};
#[allow(unused_imports)]
use futures_util::StreamExt; // required to be sourced for extension trait
use pin_project_lite::pin_project;

// Unsized window implementation - probably a better name would be value_change_filter

// TODO: this is from the futures_util crate - we should probably just use that

macro_rules! delegate_access_inner {
    ($field:ident, $inner:ty, ($($ind:tt)*)) => {
        /// Acquires a reference to the underlying sink or stream that this combinator is
        /// pulling from.
#[allow(dead_code)]
     pub fn get_ref(&self) -> &$inner {
            (&self.$field) $($ind get_ref())*
        }

        /// Acquires a mutable reference to the underlying sink or stream that this
        /// combinator is pulling from.
        ///
        /// Note that care must be taken to avoid tampering with the state of the
        /// sink or stream which may otherwise confuse this combinator.
        #[allow(dead_code)]
        pub fn get_mut(&mut self) -> &mut $inner {
            (&mut self.$field) $($ind get_mut())*
        }

        /// Acquires a pinned mutable reference to the underlying sink or stream that this
        /// combinator is pulling from.
        ///
        /// Note that care must be taken to avoid tampering with the state of the
        /// sink or stream which may otherwise confuse this combinator.
        #[allow(dead_code)]
        pub fn get_pin_mut(self: core::pin::Pin<&mut Self>) -> core::pin::Pin<&mut $inner> {
            self.project().$field $($ind get_pin_mut())*
        }

        /// Consumes this combinator, returning the underlying sink or stream.
        ///
        /// Note that this may discard intermediate state of this combinator, so
        /// care should be taken to avoid losing resources when this is called.
       #[allow(dead_code)]
       pub fn into_inner(self) -> $inner {
            self.$field $($ind into_inner())*
        }
    }
}

pin_project! {
    /// Stream for the [`window`](super::SensorFlowExt:window) method.
    #[must_use = "streams do nothing unless polled"]
    pub struct Window<St>
            where St: Stream,
 {
        #[pin]
        stream: St,
        prior_value: Option<St::Item>
    }
}

impl<St> fmt::Debug for Window<St>
where
    St: Stream,

    St: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Map").field("stream", &self.stream).finish()
    }
}

impl<St> Window<St>
where
    St: Stream,
{
    pub(crate) fn new(stream: St) -> Self {
        Self {
            stream,
            prior_value: None,
        }
    }

    delegate_access_inner!(stream, St, ());
}

impl<St> FusedStream for Window<St>
where
    St: FusedStream,
    St::Item: Clone + PartialEq,
{
    fn is_terminated(&self) -> bool {
        self.stream.is_terminated()
    }
}

impl<St> Stream for Window<St>
where
    St: Stream,
    St::Item: Clone + PartialEq,
{
    type Item = (St::Item, St::Item);

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let mut this = self.project();
        let res = ready!(this.stream.as_mut().poll_next(cx));
        let result = match res {
            Some(v) => {
                let prior_value;
                let v_copy = v.clone();
                match this.prior_value {
                    Some(pv) => {
                        // A value is available
                        prior_value = pv.clone();
                        *this.prior_value = Some(v_copy);
                        return Poll::Ready(Some((prior_value, v)));
                    }
                    None => {
                        // No value available but we're not ready to quit
                        // Tell the waker that we're elgible to be polled again
                        *this.prior_value = Some(v);
                        cx.waker().wake_by_ref();
                        return Poll::Pending;
                    }
                }
            }
            None => Poll::Ready(None),
        };
        result
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.stream.size_hint()
    }
}

// Forwarding impl of Sink from the underlying stream
#[cfg(feature = "sink")]
impl<St, Item> Sink<Item> for Window<St>
where
    St: Stream + Sink<Item>,
    F: FnMut1<St::Item>,
{
    type Error = St::Error;

    delegate_sink!(stream, Item);
}

#[cfg(test)]
mod test_super {
    // use crate::SensorFlowWindowExt;
    use futures_util::stream;
    use futures_util::stream::StreamExt;

    use crate::SensorFlowExt;

    #[tokio::test]
    async fn test_window() {
        let values = vec![0, 1, 1, 1, 2, 3, 4];
        let s = stream::iter(values);
        let s = s.window::<i32>();
        let result = s.collect::<Vec<_>>().await;
        assert_eq!(vec![(0, 1), (1, 2), (2, 3), (3, 4)], result);
    }
}
