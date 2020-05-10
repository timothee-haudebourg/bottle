use std::sync::Arc;
use std::pin::Pin;
use std::future::Future;
use parking_lot::Mutex;
use crate::{Output, Event, Handler, Remote, future};

pub(crate) trait Pending: Send {
	fn post(self: Box<Self>) -> Option<Pin<Box<dyn Future<Output = ()>>>>;

	fn process(self: Box<Self>);
}

pub(crate) struct Initialize<T, F: Send + FnOnce() -> T> {
	remote: Remote<T>,
	constructor: F
}

impl<T, F: Send + FnOnce() -> T> Initialize<T, F> {
	pub fn new(remote: Remote<T>, constructor: F) -> Initialize<T, F> {
		Initialize {
			remote, constructor
		}
	}
}

impl<T: 'static, F: 'static + Send + FnOnce() -> T> Pending for Initialize<T, F> {
	fn post(self: Box<Self>) -> Option<Pin<Box<dyn Future<Output = ()>>>> {
		let remote = self.remote.clone();
		remote.post_any(self);
		None
	}

	fn process(self: Box<Self>) {
		unsafe {
			let mut actor = self.remote.inner.actor.borrow_mut();
			actor.init((self.constructor)())
		}
	}
}

pub(crate) struct ToReceive<E: Event, T: ?Sized + Handler<E>> {
	receiver: Remote<T>,
	event: E,
	future: Arc<Mutex<future::State<T, E::Response>>>,
}

impl<E: Event, T: ?Sized + Handler<E>> ToReceive<E, T> {
	pub fn new(receiver: Remote<T>, event: E) -> ToReceive<E, T> {
		ToReceive {
			receiver: receiver.clone(),
			event,
			future: future::State::new(receiver)
		}
	}

	pub fn state(&self) -> &Arc<Mutex<future::State<T, E::Response>>> {
		&self.future
	}
}

impl<E: 'static + Event, T: 'static + ?Sized + Handler<E>> Pending for ToReceive<E, T> {
	fn post(self: Box<Self>) -> Option<Pin<Box<dyn Future<Output = ()>>>> {
		let receiver = self.receiver.clone();
		Some(Box::pin(receiver.post(self)))
	}

	fn process(self: Box<Self>) {
		let mut actor = self.receiver.inner.actor.borrow_mut();
		actor.is_busy = true;

		{
			let result = unsafe { actor.post(self.event) };
			match result {
				Output::Now(result) => {
					future::State::set(&self.future, result);
				},
				Output::Later(later) => unsafe {
					// This is safe because the actor is embedded in the future: it won't be dropped
					// until it is completed.
					future::State::pending(&self.future, later);
					return
				}
			}
		}

		actor.is_busy = false;
	}
}
