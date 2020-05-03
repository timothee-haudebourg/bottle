use std::marker::Unsize;
use std::ops::{Deref, DerefMut, DispatchFromDyn, CoerceUnsized};
use std::sync::Arc;
use crate::{Inner, Remote, Local, ThreadLocal, EventQueueRef};

pub struct Receiver<'a, T: ?Sized> {
	value: &'a mut T
}

impl<'a, T: ?Sized + Unsize<U>, U: ?Sized> CoerceUnsized<Receiver<'a, U>> for Receiver<'a, T> {}
impl<'a, T: ?Sized + Unsize<U>, U: ?Sized> DispatchFromDyn<Receiver<'a, U>> for Receiver<'a, T> {}

impl<'a, T: ?Sized> Deref for Receiver<'a, T> {
	type Target = T;

	fn deref(&self) -> &T {
		self.value
	}
}

impl<'a, T: ?Sized> DerefMut for Receiver<'a, T> {
	fn deref_mut(&mut self) -> &mut T {
		self.value
	}
}

impl<'a, T: ?Sized> Receiver<'a, T> {
	pub(crate) fn new(value: &'a mut T) -> Receiver<'a, T> {
		Receiver {
			value
		}
	}

	fn inner(&self) -> *const Inner<T> {
		unsafe {
			let ptr = self.value as *const T;

			// Find the wrapping remote's inner pointer.
			let fake_inner_ptr = ptr as *const Inner<T>;
			let inner_ptr = set_data_ptr(fake_inner_ptr, (ptr as *const u8).offset(-inner_data_offset(ptr)));

			inner_ptr
		}
	}

	pub fn as_local(&self) -> Local<T> {
		unsafe {
			// Reconstruct a wrapping Arc.
			let fake_arc = Arc::from_raw(self.inner());
			let local = Local::from_inner(fake_arc.clone()); // create an actual new Arc.
			std::mem::forget(fake_arc); // forget the fake arc.

			// We have now a fully functional Local.
			local
		}
	}

	pub fn as_remote(&self) -> Remote<T> {
		unsafe {
			// Reconstruct a wrapping Arc.
			let fake_arc = Arc::from_raw(self.inner());
			let remote = Remote::from_inner(fake_arc.clone()); // create an actual new Arc.
			std::mem::forget(fake_arc); // forget the fake arc.

			// We have now a fully functional Remote.
			remote
		}
	}
}

unsafe impl<'a, T: ?Sized> ThreadLocal for Receiver<'a, T> {
	fn queue(&self) -> &EventQueueRef {
		unsafe {
			&(&*self.inner()).queue
		}
	}
}

/// Sets the data pointer of a `?Sized` raw pointer.
///
/// For a slice/trait object, this sets the `data` field and leaves the rest
/// unchanged. For a sized raw pointer, this simply sets the pointer.
unsafe fn set_data_ptr<T: ?Sized, U>(mut ptr: *const T, data: *const U) -> *const T {
    std::ptr::write(&mut ptr as *mut _ as *mut *const u8, data as *const u8);
    ptr
}

/// Computes the offset of the data field within `Inner`.
unsafe fn inner_data_offset<T: ?Sized>(ptr: *const T) -> isize {
	// Align the unsized value to the end of the `Inner`.
	// Because it is `?Sized`, it will always be the last field in memory.
	// Note: This is a detail of the current implementation of the compiler,
	// and is not a guaranteed language detail.
	inner_data_offset_align(std::mem::align_of_val(&*ptr))
}

#[inline]
fn inner_data_offset_align(align: usize) -> isize {
    let layout = std::alloc::Layout::new::<Inner<()>>();
    (layout.size() + layout.padding_needed_for(align)) as isize
}
