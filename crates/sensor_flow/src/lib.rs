use std::{ops::Not, time::SystemTime};

#[allow(unused_imports)]
use futures::stream::StreamExt; // Required for trait extension
use futures::Stream;
mod window;

use futures_util::stream::Map;
use window::Window;

#[derive(Clone, PartialEq, Debug)]
pub struct SensorData<T> {
    pub value: T,
    pub timestamp: SystemTime,
}
// Just a helper function to ensure the streams we're returning all have the
// right implementations.
// TODO: from futures_util crate, but viz is pub(crate) so dup is necessary
#[allow(dead_code)]
pub(crate) fn assert_stream<T, S>(stream: S) -> S
where
    S: Stream<Item = T>,
{
    stream
}

impl<St> SensorFlowLogicalExt for St
where
    St: Stream,
    St::Item: std::ops::BitOr,
{
}
pub trait SensorFlowLogicalExt: Stream
where
    Self: Sized,
    <Self as Stream>::Item: std::ops::BitOr,
{
    fn or<St2>(
        self,
        other: St2,
    ) -> Map<
        futures::stream::Zip<Self, St2>,
        fn((Self::Item, St2::Item)) -> <Self::Item as std::ops::BitOr>::Output,
    >
    where
        St2: Stream,
        St2: Stream<Item = Self::Item>,
        <Self as Stream>::Item: std::ops::BitOr,
    {
        self.zip(other).map(|(a, b)| a | b)
    }

    fn and<St2>(
        self,
        other: St2,
    ) -> Map<
        futures::stream::Zip<Self, St2>,
        fn((Self::Item, St2::Item)) -> <Self::Item as std::ops::BitAnd>::Output,
    >
    where
        St2: Stream,
        St2: Stream<Item = Self::Item>,
        <Self as Stream>::Item: std::ops::BitAnd,
    {
        self.zip(other).map(|(a, b)| a & b)
    }
}

// pub trait SensorData<T> {}
// pub struct SensorDatum<T> {
//     pub value: T,
//     pub timestamp: u64,
// }

impl<St> SensorFlowNotExt for St
where
    St: Stream,
    St::Item: std::ops::Not,
    <Self as Stream>::Item: std::convert::Into<bool>,
    <Self::Item as std::ops::Not>::Output: std::convert::Into<bool>,
{
}

// Has to be its own trait b/c of the where clause that constrains <T> to an invertable value
pub trait SensorFlowNotExt: Stream
where
    <Self as Stream>::Item: std::ops::Not,
{
    /// inverts this stream's items to logical inverse of value, returning a new stream of
    /// the resulting type.
    ///
    /// Note that this function consumes the stream passed into it and returns a
    /// wrapped version of it, similar to the existing `map` methods in the
    /// standard library.
    ///
    /// See [`StreamExt::then`](Self::then) if you want to use a closure that
    /// returns a future instead of a value.
    ///
    /// # Examples
    ///
    /// ```
    /// # futures::executor::block_on(async {
    /// use futures::stream::{self, StreamExt};
    /// use sensor_flow_kata::SensorFlowNotExt;
    ///
    /// let stream = stream::iter(vec![true, false, true]);
    /// let stream = stream.not::<bool>();
    ///
    /// assert_eq!(vec![false, true, false], stream.collect::<Vec<_>>().await);
    /// # });
    /// ```
    fn not<T>(self) -> Map<Self, fn(Self::Item) -> <Self::Item as std::ops::Not>::Output>
    where
        Self: Sized,
        T: std::ops::Not,
        <Self as Stream>::Item: std::convert::Into<T>,
    {
        assert_stream(self.map(|x| !x))
    }
}

pub trait SensorFlowExt: Stream
where
    <Self as Stream>::Item: Clone + PartialEq,
{
    fn window<T>(self) -> Window<Self>
    where
        Self: Sized,
        <Self as Stream>::Item: std::convert::Into<T> + Clone + PartialEq,
    {
        assert_stream::<(Self::Item, Self::Item), _>(Window::<Self>::new(self))
    }

    fn goes_active<T>(self) -> impl Stream<Item = Self::Item>
    where
        Self: Sized,
        <Self as Stream>::Item: std::convert::Into<T> + Clone + PartialEq,
        T: std::ops::Not,
        bool: From<<<Self as Stream>::Item as std::ops::Not>::Output>,
        bool: From<<Self as Stream>::Item>,
        <Self as Stream>::Item: Not, // <Self as Stream>::Item: std::convert::Into<T> + Clone + PartialEq,
    {
        let x = self.window().filter_map(|(prev, current)| async move {
            let current_cmp = current.clone();
            if prev.not().into() && Into::<bool>::into(current) {
                Some(current_cmp)
            } else {
                None
            }
        });
        x
    }
}

impl<St> SensorFlowExt for St
where
    <Self as Stream>::Item: Clone + PartialEq,
    St: Stream,
{
}

#[cfg(test)]
mod test_flows {

    use futures_util::stream;

    use super::*;

    #[tokio::test]
    async fn test_numeric_flow_with_not_fn() {
        let values = vec![false, true, false, true];

        let s = stream::iter(values).not::<bool>();
        let result = s.collect::<Vec<_>>().await;
        assert_eq!(vec![true, false, true, false], result);
    }

    // doesn't compile (correctly b/c )
    // the trait bound `bool: From<{integer}>` is not satisfied
    // required for `{integer}` to implement `Into<bool>`
    // #[tokio::test]
    // async fn test_bad_flow_with_not_fn() {
    //     let values = vec![1, 2, 3];

    //     let s = stream::iter(values).nah::<bool>();
    //     let result = s.collect::<Vec<_>>().await;
    //     assert_eq!(vec![true, false, true, false], result);
    // }

    #[tokio::test]
    async fn test_pairs_with_zip() {
        let values = vec![false, true, true, true, false];
        let values2 = vec![true, false, true, true, false];
        let expected = vec![true, true, true, true, false];

        let s = stream::iter(values)
            .zip(stream::iter(values2))
            .map(|(x, y)| x | y);
        let result = s.collect::<Vec<_>>().await;
        assert_eq!(expected, result);
    }

    #[tokio::test]
    async fn test_pairs_with_or() {
        let values = vec![false, true, true, true, false];
        let values2 = vec![true, false, true, true, false];
        let expected = vec![true, true, true, true, false];

        let s = stream::iter(values).or(stream::iter(values2));
        let result = s.collect::<Vec<_>>().await;
        assert_eq!(expected, result);
    }

    #[tokio::test]
    async fn test_pairs_with_and() {
        let values = vec![false, true, true, true, false];
        let values2 = vec![true, false, true, true, false];
        let expected = vec![false, false, true, true, false];

        let s = stream::iter(values).and(stream::iter(values2));
        let result = s.collect::<Vec<_>>().await;
        assert_eq!(expected, result);
    }

    #[tokio::test]
    async fn test_goes_active() {
        let values = vec![false, true, true, true, false];
        let expected = vec![true];

        let s = stream::iter(values);
        let s = s.window::<bool>();
        let s = s.filter_map(|(prev, current)| async move {
            eprintln!("w: {:?} {:?}", prev, current);
            if !prev && current {
                Some(true)
            } else {
                None
            }
        });
        let result = s.collect::<Vec<bool>>().await;
        assert_eq!(expected, result);
    }

    #[tokio::test]
    async fn test_goes_active_trait() {
        let values = vec![false, true, true, true, false];
        let expected = vec![true];

        let s = stream::iter(values);
        let s = s.window::<bool>();
        let s = s.filter_map(|(prev, current)| async move {
            if !prev && current {
                Some(true)
            } else {
                None
            }
        });
        let result = s.collect::<Vec<bool>>().await;
        assert_eq!(expected, result);
    }

    // #[tokio::test]
    // async fn test_goes_active_realz() {
    //     let values = vec![false, true, true, true, false];
    //     let expected = vec![true];

    //     let s = stream::iter(values);
    //     let s = s.window::<bool>();
    //     let s = s.goes_active();
    //     // });
    //     let result = s.collect::<Vec<bool>>().await;
    //     assert_eq!(expected, result);
    // }
}
