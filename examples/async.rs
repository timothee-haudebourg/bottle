#![feature(arbitrary_self_types)]
use bottle::{Output, Receiver, Remote, Handler, EventQueue};

pub struct Foo {
	pongs: usize
}

pub enum Event {
	Ping(Remote<Foo>),
	Pong
}

impl bottle::Event for Event {
	type Response = ();
}

impl Handler<Event> for Foo {
	fn handle<'a>(mut self: Receiver<'a, Self>, event: Event) -> Output<'a, ()> {
		match event {
			Event::Ping(remote) => async move {
				println!("ping");
				remote.send(Event::Pong).await;
				self.pongs += 1;
				println!("done: {}", self.pongs)
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

	let a = Remote::new(queue.reference(), Foo { pongs: 0 });
	let b = Remote::new(queue.reference(), Foo { pongs: 0 });

	std::thread::spawn(move || {
		async_std::task::block_on(queue.process())
	});

	a.send(Event::Ping(b.clone()));
	a.send(Event::Ping(b.clone()));
	a.send(Event::Ping(b.clone()));
	a.send(Event::Ping(b)).await;
}
