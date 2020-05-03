use std::marker::Unsize;
use std::ops::{DispatchFromDyn, CoerceUnsized};
use std::sync::{Arc, Weak};
use std::cell::RefCell;
use std::mem::MaybeUninit;
use std::hash::{Hash, Hasher};
use crate::{
	Output,
	Receiver,
	Event,
	EventQueueRef,
	Handler,
	Future,
	Local,
	ThreadLocal,
	Emitter,
	SubscriptionEvent
};

pub(crate) struct Inner<T: ?Sized> {
	pub(crate) queue: EventQueueRef,
	pub(crate) actor: RefCell<T>
}

/// A pointer to a remote actor.
pub struct Remote<T: ?Sized> {
	pub(crate) inner: Arc<Inner<T>>
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Remote<U>> for Remote<T> {}
impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<Remote<U>> for Remote<T> {}

unsafe impl<T: ?Sized> Send for Remote<T> {}
unsafe impl<T: ?Sized> Sync for Remote<T> {}

impl<T: ?Sized> Remote<T> {
	pub fn from<F>(queue: EventQueueRef, constructor: F) -> Remote<T> where T: 'static + Sized, F: 'static + Send + FnOnce() -> T {
		unsafe {
			let remote = Remote {
				inner: Arc::new(Inner {
					queue: queue,
					actor: RefCell::new(
						// Why it is safe.
						// [1] We know the value won't be touched before initialization: the first
						// message received by the remote pointer is the initialization request.
						// This is because messages are processed in order in the queue. That is
						// why (among other things) posting a message directly to a remote
						// actor is unsafe.
						// [2] We know the actor won't be dropped before initialization: the
						// initialization request message holds a copy of the remote.
						MaybeUninit::uninit().assume_init()
					)
				})
			};

			remote.inner.queue.request_initialization(remote.clone(), constructor);
			remote
		}
	}

	pub fn new(queue: EventQueueRef, value: T) -> Remote<T> where T: Send + Sized {
		Remote {
			inner: Arc::new(Inner {
				queue: queue,
				actor: RefCell::new(value)
			})
		}
	}

	pub fn as_ptr(&self) -> *const T {
		self.inner.actor.as_ptr()
	}

	pub(crate) fn from_inner(inner: Arc<Inner<T>>) -> Remote<T> {
		Remote {
			inner
		}
	}

	pub fn queue(&self) -> &EventQueueRef {
		&self.inner.queue
	}

	/// Convert this pointer to a local pointer.
	///
	/// Return a local pointer to this pointer actor if `local` resides in the same thread as
	/// the pointed actor, or `None`.
	/// The `local` object is used as a proof that the conversion is valid.
	///
	/// # Safety
	/// Even if the actor resides in the local thread, it may not be initialized yet.
	/// As such, it is impossible to cast a remote pointer to a local pointer in a safe way.
	/// Caller must ensure that the remote actor has not been created with [`Remote::from`], or
	/// that it has been initialized.
	pub unsafe fn local_to<L: ThreadLocal>(&self, local: &L) -> Option<Local<T>> {
		if self.queue() == local.queue() {
			Some(Local::from_inner(self.inner.clone()))
		} else {
			None
		}
	}

	/// Must be called from the actor thread.
	pub unsafe fn post<E: Event>(&self, event: E) -> Output<E::Response> where T: 'static + Handler<E> {
		let mut actor = self.inner.actor.borrow_mut();
		let local = Receiver::new(&mut *actor);
		local.handle(event)
	}

	pub(crate) unsafe fn init(&self, mut value: T) where T: Sized {
		std::mem::swap(&mut *self.inner.actor.as_ptr(), &mut value);
		std::mem::forget(value)
	}

	pub fn send<E: Event>(&self, event: E) -> Future<E::Response> where E: 'static, T: 'static + Handler<E> {
		self.inner.queue.push(self.clone(), event)
	}

	pub fn subscribe<E: Event>(&self, subscriber: Remote<dyn Handler<E>>) -> Future<bool> where E: 'static, T: 'static + Emitter<E> {
		self.send(SubscriptionEvent::Subscribe(subscriber))
	}

	pub fn downgrade(&self) -> WeakRemote<T> {
		WeakRemote {
			inner: Arc::downgrade(&self.inner)
		}
	}
}

impl<T: ?Sized> Clone for Remote<T> {
	fn clone(&self) -> Remote<T> {
		Remote {
			inner: self.inner.clone()
		}
	}
}

impl<T: ?Sized> PartialEq for Remote<T> {
	fn eq(&self, other: &Remote<T>) -> bool {
		Arc::ptr_eq(&self.inner, &other.inner)
	}
}

impl<T: ?Sized> Eq for Remote<T> {}

impl<T: ?Sized> Hash for Remote<T> {
	fn hash<H: Hasher>(&self, h: &mut H) {
		(&*self.inner as *const Inner<T>).hash(h)
	}
}

/// A pointer to a remote actor.
pub struct WeakRemote<T: ?Sized> {
	pub(crate) inner: Weak<Inner<T>>
}

impl<T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<WeakRemote<U>> for WeakRemote<T> {}
impl<T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<WeakRemote<U>> for WeakRemote<T> {}

unsafe impl<T: ?Sized> Send for WeakRemote<T> {}
unsafe impl<T: ?Sized> Sync for WeakRemote<T> {}

impl<T: ?Sized> WeakRemote<T> {
	pub fn upgrade(&self) -> Option<Remote<T>> {
		if let Some(inner) = self.inner.upgrade() {
			Some(Remote {
				inner
			})
		} else {
			None
		}
	}

	pub fn send<E: Event>(&self, event: E) -> Option<Future<E::Response>> where E: 'static, T: 'static + Handler<E> {
		if let Some(remote) = self.upgrade() {
			Some(remote.send(event))
		} else {
			None
		}
	}
}

impl<T: ?Sized> Clone for WeakRemote<T> {
	fn clone(&self) -> WeakRemote<T> {
		WeakRemote {
			inner: self.inner.clone()
		}
	}
}

impl<T: ?Sized> PartialEq for WeakRemote<T> {
	fn eq(&self, other: &WeakRemote<T>) -> bool {
		Weak::ptr_eq(&self.inner, &other.inner)
	}
}

impl<T: ?Sized> Eq for WeakRemote<T> {}
