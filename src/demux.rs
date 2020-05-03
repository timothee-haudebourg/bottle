use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use parking_lot::Mutex;
use crate::{Event, Remote, WeakRemote, Handler};

#[derive(Clone)]
struct Receiver<E: Event> {
	remote: WeakRemote<dyn Handler<E>>,
	ptr: *const dyn Handler<E>
}

unsafe impl<E: Event> Send for Receiver<E> {}

impl<E: Event> Receiver<E> {
	fn new(actor: &Remote<dyn Handler<E>>) -> Receiver<E> {
		Receiver {
			remote: actor.downgrade(),
			ptr: actor.as_ptr()
		}
	}
}

impl<E: Event> PartialEq for Receiver<E> {
	fn eq(&self, other: &Receiver<E>) -> bool {
		self.ptr == other.ptr
	}
}

impl<E: Event> Eq for Receiver<E> {}

impl<E: Event> Hash for Receiver<E> {
	fn hash<H: Hasher>(&self, h: &mut H) {
		self.ptr.hash(h)
	}
}

pub struct Demux<E: Event> {
	subscribers: Mutex<HashSet<Receiver<E>>>
}

impl<E: Event> Demux<E> {
	pub fn new() -> Demux<E> {
		Demux {
			subscribers: Mutex::new(HashSet::new())
		}
	}

	pub fn subscribe(&mut self, actor: &Remote<dyn Handler<E>>) -> bool {
		let mut subscribers = self.subscribers.lock();
		subscribers.insert(Receiver::new(actor))
	}

	pub fn unsubscribe(&mut self, actor: &Remote<dyn Handler<E>>) -> bool {
		let mut subscribers = self.subscribers.lock();
		subscribers.remove(&Receiver::new(actor))
	}

	pub fn send(&self, event: E) where E: 'static + Clone {
		let mut subscribers = self.subscribers.lock();
		let mut to_remove = Vec::new();
		for subscriber in subscribers.iter() {
			if let Some(subscriber) = subscriber.remote.upgrade() {
				subscriber.send(event.clone());
			} else {
				to_remove.push(subscriber.clone())
			}
		}

		for dead_subscriber in &to_remove {
			subscribers.remove(dead_subscriber);
		}
	}
}
