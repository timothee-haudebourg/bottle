#![feature(
	// To allow the use of `Receiver<Self>` as `self` type.
	arbitrary_self_types,

	// To create smart pointers `Remote`, `Local` and `Receiver`
	unsize,
	coerce_unsized,
	dispatch_from_dyn,

	// To wrap futures in Future.
	box_into_pin,

	// To convert `Receiver` into `Local` and `Remote` using black magic.
	alloc_layout_extra,

	// To avoid `Send` and `Sync` auto implementation.
	negative_impls,

	// To implement `Hash` for `WeakRemote`.
	weak_into_raw
)]

#[macro_use]
extern crate static_assertions;

use std::pin::Pin;
use futures::future::FutureExt;

mod future;
mod receiver;
mod remote;
mod local;
mod pending;
mod queue;
mod demux;
mod emitter;

pub use future::Future;
pub use receiver::*;
pub use remote::*;
pub use local::*;
pub use pending::*;
pub use queue::*;
pub use demux::*;
pub use emitter::*;

pub trait Event: Send {
	type Response: Send;
}

pub enum Output<T> {
	Now(T),
	Later(Pin<Box<dyn Send + std::future::Future<Output = T>>>)
}

impl<T, F: 'static + Send + std::future::Future<Output = T>> From<F> for Output<T> {
	fn from(future: F) -> Output<T> {
		Output::Later(future.boxed())
	}
}

pub trait Handler<E: Event> {
	fn handle(self: Receiver<Self>, event: E) -> Output<E::Response>;
}

/// A trait for thread local values, attached to an `EventQueue`.
pub unsafe trait ThreadLocal {
	fn queue(&self) -> &EventQueueRef;
}
