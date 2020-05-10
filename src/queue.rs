use std::sync::Arc;
use std::future::Future as StdFuture;
use std::task::{Waker, Context, Poll};
use std::pin::Pin;
use crossbeam_queue::SegQueue as AtomicQueue;
use parking_lot::Mutex;
use crate::{Event, Remote, Handler, Pending, Future, ToReceive, Initialize};

pub struct Queue<T> {
	inner: AtomicQueue<T>,
	waker: Mutex<Option<Waker>>
}

impl<T> Queue<T> {
	pub fn new() -> Queue<T> {
		Queue {
			inner: AtomicQueue::new(),
			waker: Mutex::new(None)
		}
	}

	pub fn push(&self, value: T) {
		self.inner.push(value);

		let mut waker = None;
		if let Some(mut locked_waker) = self.waker.try_lock() {
			std::mem::swap(&mut waker, &mut locked_waker);
		}

		if let Some(waker) = waker {
			waker.wake();
		}
	}

	pub fn pop(&self, new_waker: Waker) -> Option<T> {
		{
			// We update the waker *before* the pop so that it is available to any push.
			let mut waker = self.waker.lock();
			*waker = Some(new_waker);
		}

		if let Ok(value) = self.inner.pop() {
			let mut waker = None;
			std::mem::swap(&mut waker, &mut self.waker.lock());
			if let Some(waker) = waker {
				waker.wake();
			}

			Some(value)
		} else {
			// We don't need to call a waker here.
			// If a push occured after this pop, the waker was available and waked.
			None
		}
	}
}

/// A reference to an event queue.
#[derive(Clone)]
pub struct EventQueueRef {
	queue: Arc<Queue<Box<dyn Pending>>>
}

impl EventQueueRef {
	/// Push an event to the queue.
	pub fn push<E: 'static + Event, T: 'static + ?Sized + Handler<E>>(&self, receiver: Remote<T>, event: E) -> Future<T, E::Response> {
		let pending = Box::new(ToReceive::new(receiver, event));
		let future = Future::new(pending.state().clone());
		self.queue.push(pending);
		future
	}

	pub(crate) unsafe fn request_initialization<T: 'static, F: 'static>(&self, remote: Remote<T>, constructor: F) where F: Send + FnOnce() -> T {
		self.queue.push(Box::new(Initialize::new(remote, constructor)));
	}
}

impl PartialEq for EventQueueRef {
	fn eq(&self, other: &EventQueueRef) -> bool {
		Arc::ptr_eq(&self.queue, &other.queue)
	}
}

impl Eq for EventQueueRef {}

pub struct EventQueue {
	queue: Arc<Queue<Box<dyn Pending>>>
}

impl !Sync for EventQueue {}
assert_not_impl_any!(EventQueue: Sync);

impl EventQueue {
	pub fn new() -> EventQueue {
		EventQueue {
			queue: Arc::new(Queue::new())
		}
	}

	pub fn reference(&self) -> EventQueueRef {
		EventQueueRef {
			queue: self.queue.clone()
		}
	}

	pub fn process(self) -> EventQueueProcessor {
		EventQueueProcessor {
			queue: self.queue,
			pending_futures: Vec::new()
		}
	}
}

/// Event Queue Processor.
///
/// This is the object in charge of processin a queue and actually posting the events to the
/// actors.
///
/// # Thread Safety
/// Since every actor attached to the processor's queue must be run in the same thread and never
/// move (which is the basis of the actor model), this type does not implement `Send` nor `Sync`.
pub struct EventQueueProcessor {
	queue: Arc<Queue<Box<dyn Pending>>>,
	pending_futures: Vec<Pin<Box<dyn StdFuture<Output = ()>>>>
}

impl !Send for EventQueueProcessor {}
impl !Sync for EventQueueProcessor {}
assert_not_impl_any!(EventQueueProcessor: Send, Sync);

impl EventQueueProcessor {
	pub fn reference(&self) -> EventQueueRef {
		EventQueueRef {
			queue: self.queue.clone()
		}
	}
}

impl futures::future::Future for EventQueueProcessor {
	type Output = ();

	fn poll(mut self: Pin<&mut Self>, ctx: &mut Context) -> Poll<()> {
		retain_mut(&mut self.pending_futures, |future| {
			match future.as_mut().poll(ctx) {
				Poll::Pending => true,
				_ => false
			}
		});

		if let Some(pending) = self.queue.pop(ctx.waker().clone()) {
			if let Some(mut future) = pending.post() {
				match future.as_mut().poll(ctx) {
					Poll::Pending => {
						self.pending_futures.push(future);
					},
					_ => ()
				}
			}
		}

		Poll::Pending
	}
}

fn retain_mut<T, F>(vec: &mut Vec<T>, mut f: F)
where
	F: FnMut(&mut T) -> bool,
{
	let len = vec.len();
	let mut del = 0;
	{
		let v = &mut **vec;

		for i in 0..len {
			if !f(&mut v[i]) {
				del += 1;
			} else if del > 0 {
				v.swap(i - del, i);
			}
		}
	}
	if del > 0 {
		vec.truncate(len - del);
	}
}
