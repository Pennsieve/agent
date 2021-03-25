//! Various utility functions for use with Rust futures

use futures::{self, *};

// This basically converts a concrete object implementing the `Future` trait
// into a `Box`ed trait object. This allows for a function to return a variety
// of Future-traited objects with different concrete types, while allow them
// all to be typed the same.
//
// Downside: this function introduces a heap allocated value to accomplish this
// until `impl traits` are available in the stable rustc channel.
//
// See https://github.com/rust-lang/rust/issues/34511 for tracking the status
// of `impl traits`.
pub fn to_future_trait<F, I, E>(f: F) -> Box<dyn Future<Item = I, Error = E> + Send>
where
    F: 'static + Send + Future<Item = I, Error = E>,
{
    Box::new(f)
}

pub fn to_stream_trait<S, I, E>(s: S) -> Box<dyn Stream<Item = I, Error = E> + Send>
where
    S: 'static + Send + Stream<Item = I, Error = E>,
{
    Box::new(s)
}

pub trait PSFuture<T, E>
where
    E: ::failure::Fail,
{
    fn into_trait(self) -> Box<dyn Future<Item = T, Error = E> + Send>
    where
        Self: 'static + Send + Sized + future::Future<Item = T, Error = E>,
    {
        Box::new(self)
    }
}

impl<T, E> PSFuture<T, E> for Box<dyn Future<Item = T, Error = E> + Send> where E: ::failure::Fail {}

impl<T, E> PSFuture<T, E> for dyn future::Future<Item = T, Error = E> where E: ::failure::Fail {}

impl<T, E> PSFuture<T, E> for future::FutureResult<T, E> where E: ::failure::Fail {}

impl<T, E> PSFuture<T, E> for future::Empty<T, E> where E: ::failure::Fail {}

impl<T, F, E> PSFuture<T, E> for future::PollFn<F>
where
    E: ::failure::Fail,
    F: FnMut() -> futures::Poll<T, E>,
{
}

impl<T, E, A, F> PSFuture<T, E> for future::Map<A, F>
where
    E: ::failure::Fail,
    A: future::Future<Error = E>,
    F: FnOnce(A::Item) -> T,
{
}

impl<T, E, K, A, F> PSFuture<T, K> for future::MapErr<A, F>
where
    E: ::failure::Fail,
    K: ::failure::Fail,
    A: future::Future<Error = E>,
    F: FnOnce(A::Error) -> K,
{
}

impl<T, E, A, B, F> PSFuture<T, E> for future::AndThen<A, B, F>
where
    E: ::failure::Fail,
    A: future::Future<Error = E>,
    B: IntoFuture<Error = A::Error>,
    F: FnOnce(A::Item) -> B,
{
}

impl<T, E, A, B, F> PSFuture<T, E> for future::Then<A, B, F>
where
    E: ::failure::Fail,
    A: future::Future<Error = E>,
    B: IntoFuture<Error = A::Error>,
    F: FnOnce(Result<A::Item, A::Error>) -> B,
{
}

impl<T, E, A, B, F> PSFuture<T, E> for future::OrElse<A, B, F>
where
    E: ::failure::Fail,
    A: Future,
    B: IntoFuture<Item = A::Item>,
    F: FnOnce(A::Error) -> B,
{
}

impl<T, E, F, R> PSFuture<T, E> for future::Lazy<F, R>
where
    E: ::failure::Fail,
    F: FnOnce() -> R,
    R: IntoFuture<Error = E>,
{
}

impl<T, E, A, B> PSFuture<T, E> for future::Select<A, B>
where
    E: ::failure::Fail,
    A: Future<Error = E>,
    B: Future<Item = A::Item, Error = A::Error>,
{
}

impl<T, E, A, B> PSFuture<T, E> for future::Select2<A, B>
where
    E: ::failure::Fail,
    A: Future<Error = E>,
    B: Future<Error = E>,
{
}

impl<T, E, S, F, G> PSFuture<T, E> for stream::Fold<S, F, G, T>
where
    E: ::failure::Fail,
    S: Stream<Error = E>,
    F: FnMut(T, S::Item) -> G,
    G: IntoFuture<Item = T>,
    S::Error: From<G::Error>,
{
}
