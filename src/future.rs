use std::sync::Arc;
use std::pin::Pin;
use std::task::{Waker, Context, Poll};
use parking_lot::Mutex;
use crate::Remote;

// pub(crate) struct State<T> {
// 	result: Option<T>,
// 	waker: Option<Waker>
// }
//
// impl<T> State<T> {
// 	pub fn set(state: Arc<Mutex<State<T>>>, value: T) {
// 		let mut state = state.lock();
// 		state.result = Some(value);
//
// 		let mut waker = None;
// 		std::mem::swap(&mut waker, &mut state.waker);
// 		if let Some(waker) = waker {
// 			waker.wake()
// 		}
// 	}
// }

pub struct Future<R: ?Sized, T: 'static + Send> {
	pub(crate) state: Arc<Mutex<State<R, T>>>
}

impl<R: ?Sized, T: 'static + Send> Future<R, T> {
	pub(crate) fn new(state: Arc<Mutex<State<R, T>>>) -> Future<R, T> {
		Future {
			state
		}
	}

	//
	// pub fn from_future(future: Box<dyn Send + std::future::Future<Output = T>>) -> Future<T> {
	// 	Future {
	// 		state: Arc::new(Mutex::new(State {
	// 			result: Some(FutureResult::Later(Box::into_pin(future))),
	// 			waker: None
	// 		}))
	// 	}
	// }
}

impl<R: ?Sized, T: 'static + Send> futures::future::Future for Future<R, T> {
	type Output = T;

	fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<T> {
		let mut state = self.state.lock();
		state.waker = None;
		let mut result = None;
		std::mem::swap(&mut result, &mut state.result);
		match result {
			Some(result) => {
				Poll::Ready(result)
			},
			// Some(FutureResult::Later(mut future)) => {
			// 	let poll = Pin::as_mut(&mut future).poll(ctx);
			// 	state.result = Some(FutureResult::Later(future));
			// 	poll
			// },
			None => {
				state.waker = Some(ctx.waker().clone());
				Poll::Pending
			}
		}
	}
}

pub(crate) struct State<R: ?Sized, T: 'static + Send> {
	remote: Remote<R>,
	result: Option<T>,
	waker: Option<Waker>,
	local_waker: Option<Waker>,
	local_future: Option<Pin<Box<dyn 'static + std::future::Future<Output = T>>>>
}

unsafe impl<R: ?Sized, T: 'static + Send> Send for State<R, T> {}

impl<R: ?Sized, T: 'static + Send> State<R, T> {
	pub fn new(remote: Remote<R>) -> Arc<Mutex<State<R, T>>> {
		Arc::new(Mutex::new(State {
			remote,
			result: None,
			waker: None,
			local_waker: None,
			local_future: None
		}))
	}

	pub fn set(state: &Arc<Mutex<State<R, T>>>, value: T) {
		let mut state = state.lock();
		state.result = Some(value);

		let mut waker = None;
		std::mem::swap(&mut waker, &mut state.waker);
		if let Some(waker) = waker {
			waker.wake()
		}

		let mut local_waker = None;
		std::mem::swap(&mut local_waker, &mut state.local_waker);
		if let Some(local_waker) = local_waker {
			local_waker.wake()
		}
	}

	// The future lifetime must be bound to the receiver lifetime.
	pub unsafe fn pending<'a, F: 'a + std::future::Future<Output = T>>(state: &Arc<Mutex<State<R, T>>>, future: F) {
		let mut state = state.lock();
		state.local_future = Some({ // unsafe part
			// This is safe because the receiver won't be dropped until the future is completed.
			std::mem::transmute(Box::pin(future) as Pin<Box<dyn 'a + std::future::Future<Output = T>>>)
		});

		let mut local_waker = None;
		std::mem::swap(&mut local_waker, &mut state.local_waker);
		if let Some(local_waker) = local_waker {
			local_waker.wake()
		}
	}
}

pub(crate) struct LocalFuture<R: ?Sized, T: 'static + Send> {
	state: Arc<Mutex<State<R, T>>>
}

impl<R: ?Sized, T: 'static + Send> LocalFuture<R, T> {
	pub fn new(future_state: Arc<Mutex<State<R, T>>>) -> LocalFuture<R, T> {
		LocalFuture {
			state: future_state
		}
	}
}

impl<R: ?Sized, T: 'static + Send> std::future::Future for LocalFuture<R, T> {
	type Output = ();

	fn poll(self: Pin<&mut Self>, ctx: &mut Context) -> Poll<()> {
		let mut state = self.state.lock();

		if state.local_future.is_some() {
			state.local_waker = None;
			match state.local_future.as_mut().unwrap().as_mut().poll(ctx) {
				Poll::Pending => Poll::Pending,
				Poll::Ready(result) => {
					state.result = Some(result);

					let mut waker = None;
					std::mem::swap(&mut waker, &mut state.waker);
					if let Some(waker) = waker {
						waker.wake()
					}

					unsafe {
						state.remote.restart();
					}

					Poll::Ready(())
				}
			}
		} else {
			state.local_waker = Some(ctx.waker().clone());
			Poll::Pending
		}
	}
}
