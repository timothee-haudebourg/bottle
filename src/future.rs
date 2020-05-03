use std::sync::Arc;
use std::pin::Pin;
use std::task::{Waker, Context, Poll};
use parking_lot::Mutex;

enum FutureResult<T> {
	Now(T),
	Later(Pin<Box<dyn Send + futures::future::Future<Output = T>>>)
}

pub(crate) struct State<T> {
	result: Option<FutureResult<T>>,
	waker: Option<Waker>
}

impl<T> State<T> {
	pub fn set_now(state: Arc<Mutex<State<T>>>, value: T) {
		let mut state = state.lock();
		state.result = Some(FutureResult::Now(value));

		let mut waker = None;
		std::mem::swap(&mut waker, &mut state.waker);
		if let Some(waker) = waker {
			waker.wake()
		}
	}

	pub fn set_later(state: Arc<Mutex<State<T>>>, future: Pin<Box<dyn Send + std::future::Future<Output = T>>>) {
		let mut state = state.lock();
		state.result = Some(FutureResult::Later(future));

		let mut waker = None;
		std::mem::swap(&mut waker, &mut state.waker);
		if let Some(waker) = waker {
			waker.wake()
		}
	}
}

pub struct Future<T> {
	pub(crate) state: Arc<Mutex<State<T>>>
}

impl<T> Future<T> {
	pub fn new() -> Future<T> {
		Future {
			state: Arc::new(Mutex::new(State {
				result: None,
				waker: None
			}))
		}
	}

	pub fn from_future(future: Box<dyn Send + std::future::Future<Output = T>>) -> Future<T> {
		Future {
			state: Arc::new(Mutex::new(State {
				result: Some(FutureResult::Later(Box::into_pin(future))),
				waker: None
			}))
		}
	}
}

impl<T> futures::future::Future for Future<T> {
	type Output = T;

	fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<T> {
		let mut state = self.state.lock();

		state.waker = None;
		let mut result = None;
		std::mem::swap(&mut result, &mut state.result);
		match result {
			Some(FutureResult::Now(result)) => Poll::Ready(result),
			Some(FutureResult::Later(mut future)) => {
				let poll = Pin::as_mut(&mut future).poll(ctx);
				state.result = Some(FutureResult::Later(future));
				poll
			},
			None => {
				state.waker = Some(ctx.waker().clone());
				Poll::Pending
			}
		}
	}
}
