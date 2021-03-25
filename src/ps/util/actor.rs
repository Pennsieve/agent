/// Convenience functions for working with actors in actix.
use actix::prelude::*;

/// Send a message, without regard for whether the target will receive it.
/// See `actix::Addr/do_send`.
pub fn send_unconditionally<A, M>(message: M)
where
    M: 'static + Message + Send,
    M::Result: Send,
    A: Actor + SystemService + Handler<M>,
{
    System::current().registry().get::<A>().do_send(message)
}
