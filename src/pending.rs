use std::sync::Weak;
use crate::{Output, Event, Handler, Remote, future};
use parking_lot::Mutex;

pub trait Pending: Send {
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

impl<T, F: Send + FnOnce() -> T> Pending for Initialize<T, F> {
	fn process(self: Box<Self>) {
		unsafe {
			self.remote.init((self.constructor)())
		}
	}
}

pub(crate) struct ToReceive<E: Event, T: ?Sized + Handler<E>> {
	receiver: Remote<T>,
	event: E,
	future: Weak<Mutex<future::State<E::Response>>>
}

impl<E: Event, T: ?Sized + Handler<E>> ToReceive<E, T> {
	pub fn new(receiver: Remote<T>, event: E, future: Weak<Mutex<future::State<E::Response>>>) -> ToReceive<E, T> {
		ToReceive {
			receiver, event, future
		}
	}
}

impl<E: 'static + Event, T: 'static + ?Sized + Handler<E>> Pending for ToReceive<E, T> {
	fn process(self: Box<Self>) {
		let result = unsafe { self.receiver.post(self.event) };
		if let Some(future) = self.future.upgrade() {
			match result {
				Output::Now(result) => future::State::set_now(future, result),
				Output::Later(later) => future::State::set_later(future, later)
			}
		}
	}
}
