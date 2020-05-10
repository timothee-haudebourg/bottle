use std::marker::Unsize;
use std::ops::{DispatchFromDyn, CoerceUnsized};
use std::sync::Arc;
use crate::{Remote, Event, Handler, Future, Inner, ThreadLocal, EventQueueRef};

/// A reference to a local actor.
///
/// Local references are accessible when the referenced actor resides in the same thread as the
/// current actor.
/// A Local reference acts as a [`RefCell`].
pub struct Local<T: ?Sized> {
	inner: Arc<Inner<T>>
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Local<U>> for Local<T> {}
impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<Local<U>> for Local<T> {}

impl<T: ?Sized> !Send for Local<T> {}
impl<T: ?Sized> !Sync for Local<T> {}
assert_not_impl_any!(Local<()>: Send, Sync);

impl<T: ?Sized> Clone for Local<T> {
	fn clone(&self) -> Local<T> {
		Local {
			inner: self.inner.clone()
		}
	}
}

impl<T: ?Sized> Local<T> {
	pub(crate) fn from_inner(inner: Arc<Inner<T>>) -> Local<T> {
		Local {
			inner
		}
	}

	pub fn as_remote(&self) -> Remote<T> {
		Remote::from_inner(self.inner.clone())
	}

	pub fn send<E: Event>(&self, event: E) -> Future<T, E::Response> where E: 'static, T: 'static + Handler<E> {
		self.inner.queue.push(self.as_remote(), event)
	}
}

unsafe impl<T: ?Sized> ThreadLocal for Local<T> {
	fn queue(&self) -> &EventQueueRef {
		&self.inner.queue
	}
}
