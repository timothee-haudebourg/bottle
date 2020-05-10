use crate::{Event, Remote, Handler, Receiver, Output};

pub trait Emitter<E: Event> {
	fn subscribe(&mut self, remote: Remote<dyn Handler<E>>) -> bool;

	fn unsubscribe(&mut self, remote: Remote<dyn Handler<E>>) -> bool;
}

pub enum SubscriptionEvent<E> {
	Subscribe(Remote<dyn Handler<E>>),
	Unsubscribe(Remote<dyn Handler<E>>)
}

impl<E: Event> Event for SubscriptionEvent<E> {
	type Response = bool;
}

impl<E: Event, T: ?Sized + Emitter<E>> Handler<SubscriptionEvent<E>> for T {
	fn handle<'a>(mut self: Receiver<'a, Self>, event: SubscriptionEvent<E>) -> Output<'a, bool> {
		match event {
			SubscriptionEvent::Subscribe(remote) => Output::Now(self.subscribe(remote)),
			SubscriptionEvent::Unsubscribe(remote) => Output::Now(self.unsubscribe(remote))
		}
	}
}

#[macro_export]
macro_rules! emitter (
	( $type:ty { $($field:ident : $event_type:ty),* } ) => {
		$(
			emitter_impl!($type, $field, $event_type);
		)*
	}
);

#[macro_export]
macro_rules! emitter_impl {
	($type:ty, $field:ident, $event_type:ty) => {
		impl ::bottle::Emitter<$event_type> for $type {
			fn subscribe(&mut self, remote: ::bottle::Remote<dyn ::bottle::Handler<$event_type>>) -> bool {
				self.$field.subscribe(&remote)
			}

			fn unsubscribe(&mut self, remote: ::bottle::Remote<dyn ::bottle::Handler<$event_type>>) -> bool {
				self.$field.unsubscribe(&remote)
			}
		}
	}
}
