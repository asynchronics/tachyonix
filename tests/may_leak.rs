use std::mem::{self, ManuallyDrop};
use std::pin::Pin;

use futures_executor::block_on;
use futures_util::poll;

use tachyonix::channel;

// Forget a send future, then drop the sender.
//
// Mainly meant for MIRI. See <https://github.com/asynchronics/tachyonix/issues/1>.
#[test]
fn forget_send_future_drop_sender() {
    let (s, mut r) = channel(1);

    // Boxing so that MIRI can identify invalidated memory access via use-after-free.
    let s = Box::new(s);

    s.try_send(13).unwrap();

    let mut s_fut = ManuallyDrop::new(s.send(42));
    let mut s_fut = unsafe { Pin::new_unchecked(&mut *s_fut) }; // safe: the unpinned future is shadowed.
    assert!(block_on(async { poll!(s_fut.as_mut()) }).is_pending());

    std::mem::forget(s_fut);

    drop(s);

    assert!(r.try_recv().is_ok());
}

// Forget a send future, then forget the sender.
//
// Mainly meant for MIRI. See <https://github.com/asynchronics/tachyonix/issues/1>.
#[test]
fn forget_send_future_forget_sender() {
    let (s, mut r) = channel(1);

    let mut s = ManuallyDrop::new(s);
    s.try_send(13).unwrap();

    let mut s_fut = ManuallyDrop::new(s.send(42));
    let mut s_fut = unsafe { Pin::new_unchecked(&mut *s_fut) }; // safe: the unpinned future is shadowed.
    assert!(block_on(async { poll!(s_fut.as_mut()) }).is_pending());

    mem::forget(s_fut);

    // Forget the sender and move in a new sender.
    s = ManuallyDrop::new(tachyonix::channel(1).0);
    let _ = s;

    assert!(r.try_recv().is_ok());
}

// Forget a send future, then reuse the sender.
//
// See <https://github.com/asynchronics/tachyonix/issues/1>.
#[test]
fn forget_send_future_reuse_sender() {
    let (s, mut r) = channel(1);

    let s = ManuallyDrop::new(s);
    s.try_send(7).unwrap();

    let mut s_fut1 = ManuallyDrop::new(s.send(13));
    let mut s_fut1 = unsafe { Pin::new_unchecked(&mut *s_fut1) }; // safe: the unpinned future is shadowed.
    assert!(block_on(async { poll!(s_fut1.as_mut()) }).is_pending());

    mem::forget(s_fut1);

    assert!(block_on(async { poll!(Box::pin(s.send(42))) }).is_pending());

    assert!(r.try_recv().is_ok());
}
