#![feature(arbitrary_self_types)]
use bottle::{Output, Receiver, Remote, Handler, EventQueue};

pub struct Foo {
	//
}

pub enum Event {
	Ping(Remote<Foo>),
	Pong
}

impl bottle::Event for Event {
	type Response = ();
}

impl Handler<Event> for Foo {
	fn handle(self: Receiver<Self>, event: Event) -> Output<()> {
		match event {
			Event::Ping(remote) => async move {
				println!("ping");
				remote.send(Event::Pong).await
			}.into(),
			Event::Pong => {
				println!("pong");
				Output::Now(())
			}
		}
	}
}

#[async_std::main]
async fn main() {
	let queue = EventQueue::new();

	let a = Remote::new(queue.reference(), Foo { });
	let b = Remote::new(queue.reference(), Foo { });

	std::thread::spawn(move || {
		async_std::task::block_on(queue.process())
	});

	a.send(Event::Ping(b)).await;
}
