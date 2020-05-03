#![feature(arbitrary_self_types)]
use bottle::{Receiver, Remote, Handler, EventQueue};

struct Foo {
	value: i32
}

struct Reflect;

impl bottle::Event for Reflect {
	type Response = Remote<dyn Handler<Event>>;
}

enum Event {
	Foo
}

impl bottle::Event for Event {
	type Response = ();
}

impl Handler<Reflect> for Foo {
	fn handle(self: Receiver<Self>, _event: Reflect) -> Remote<dyn Handler<Event>> {
		let remote = self.as_remote();
		remote as Remote<dyn Handler<Event>>
	}
}

impl Handler<Event> for Foo {
	fn handle(self: Receiver<Self>, _event: Event) {
		println!("my value is {}", self.value)
	}
}

#[async_std::main]
async fn main() {
	let queue = EventQueue::new();
	let foo = Remote::new(queue.reference(), Foo { value: 42 });

	std::thread::spawn(move || {
		async_std::task::block_on(queue.process())
	});

	let remote = foo.send(Reflect).await;
	remote.send(Event::Foo).await;
}
