# Bottle

<table><tr>
  <td><a href="https://docs.rs/bottle">Documentation</a></td>
  <td><a href="https://crates.io/crates/bottle">Crate informations</a></td>
  <td><a href="https://github.com/timothee-haudebourg/bottle">Repository</a></td>
</tr></table>

An actor model implementation for Rust.

## Basic usage

```rust
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
```

## License

Licensed under either of

 * Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any
additional terms or conditions.
