#![feature(arbitrary_self_types)]

#[macro_use]
extern crate bottle;
use bottle::{Output, Receiver, Remote, Handler, EventQueue, Demux};

struct Foo {
	event1_demux: Demux<Event1>,
	event2_demux: Demux<Event2>
}

#[derive(Clone, Copy)]
struct Event1;

impl bottle::Event for Event1 {
	type Response = ();
}

#[derive(Clone, Copy)]
struct Event2;

impl bottle::Event for Event2 {
	type Response = ();
}

emitter! {
	Foo {
		event1_demux: Event1,
		event2_demux: Event2
	}
}

struct Emit;

impl bottle::Event for Emit {
	type Response = ();
}

impl Handler<Emit> for Foo {
	fn handle(self: Receiver<Self>, _event: Emit) -> Output<()> {
		println!("emit!");

		self.event1_demux.send(Event1);
		self.event2_demux.send(Event2);

		Output::Now(())
	}
}

struct Bar {
	// ...
}

impl Handler<Event1> for Bar {
	fn handle(self: Receiver<Self>, _event: Event1) -> Output<()> {
		println!("received!");
		Output::Now(())
	}
}

fn main() {
	let queue = EventQueue::new();
	let foo = Remote::new(queue.reference(), Foo {
		event1_demux: Demux::new(),
		event2_demux: Demux::new()
	});

	let rec1 = Remote::new(queue.reference(), Bar {});
	let rec2 = Remote::new(queue.reference(), Bar {});

	foo.subscribe(rec1.clone());
	foo.subscribe(rec2.clone());
	foo.send(Emit);

	async_std::task::block_on(queue.process());
}
