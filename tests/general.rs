//! Note: timer-based tests are disabled for MIRI.

#[cfg(not(miri))]
use std::future::Future;
#[cfg(not(miri))]
use std::task::{Context, Poll};
use std::thread;
#[cfg(not(miri))]
use std::time::Duration;

use futures_executor::block_on;
#[cfg(not(miri))]
use futures_task::noop_waker;
#[cfg(not(miri))]
use futures_util::pin_mut;
use tachyonix::{channel, RecvError, SendError, TryRecvError, TrySendError};

// Sleep for the provided number of milliseconds.
#[cfg(not(miri))]
fn sleep(millis: u64) {
    thread::sleep(Duration::from_millis(millis));
}

// Poll the future once and keep it alive for the specified number of
// milliseconds.
#[cfg(not(miri))]
fn poll_once_and_keep_alive<F: Future>(f: F, millis: u64) -> Poll<F::Output> {
    pin_mut!(f);
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);

    let res = f.poll(&mut cx);

    // Delay the drop of the original (shadowed) future.
    sleep(millis);

    res
}

// Basic synchronous sending/receiving functionality.
#[cfg(not(miri))]
#[test]
fn try_send_recv() {
    let (mut s, mut r) = channel(2);

    let th_send = thread::spawn(move || {
        sleep(100);
        assert_eq!(s.try_send(3), Ok(())); // t = t0 + 100
        assert_eq!(s.try_send(7), Ok(())); // t = t0 + 100
        assert_eq!(s.try_send(13), Err(TrySendError::Full(13))); // t = t0 + 100
        sleep(200);
        assert_eq!(s.try_send(42), Ok(())); // t = t0 + 300
    });

    sleep(200);
    assert_eq!(r.try_recv(), Ok(3)); // t = t0 + 200
    assert_eq!(r.try_recv(), Ok(7)); // t = t0 + 200
    assert_eq!(r.try_recv(), Err(TryRecvError::Empty)); // t = t0 + 200
    sleep(200);
    assert_eq!(r.try_recv(), Ok(42)); // t = t0 + 400
    assert_eq!(r.try_recv(), Err(TryRecvError::Closed)); // t = t0 + 400

    th_send.join().unwrap();
}

// Basic asynchronous sending functionality.
#[cfg(not(miri))]
#[test]
fn async_send() {
    let (mut s, mut r) = channel(2);

    let th_send = thread::spawn(move || {
        block_on(s.send(3)).unwrap();
        block_on(s.send(7)).unwrap();
        block_on(s.send(13)).unwrap(); // blocked until t0 + 300
        sleep(200);
        block_on(s.send(42)).unwrap(); // t = t0 + 500
    });

    sleep(300);
    assert_eq!(r.try_recv(), Ok(3)); // t = t0 + 300
    assert_eq!(r.try_recv(), Ok(7)); // t = t0 + 300
    sleep(100);
    assert_eq!(r.try_recv(), Ok(13)); // t = t0 + 400
    sleep(200);
    assert_eq!(r.try_recv(), Ok(42)); // t = t0 + 600

    th_send.join().unwrap();
}

// Basic asynchronous receiving functionality.
#[cfg(not(miri))]
#[test]
fn async_recv() {
    let (mut s, mut r) = channel(100);

    let th_send = thread::spawn(move || {
        sleep(100);
        assert_eq!(s.try_send(3), Ok(())); // t = t0 + 100
        assert_eq!(s.try_send(7), Ok(())); // t = t0 + 100
        assert_eq!(s.try_send(42), Ok(())); // t = t0 + 100
        sleep(100);
    });

    assert_eq!(r.try_recv(), Err(TryRecvError::Empty)); // t = t0
    assert_eq!(block_on(r.recv()), Ok(3)); // blocked from t0 to t0 + 100
    assert_eq!(block_on(r.recv()), Ok(7)); // t = t0 + 100
    assert_eq!(block_on(r.recv()), Ok(42)); // t = t0 + 100
    assert_eq!(r.try_recv(), Err(TryRecvError::Empty)); // t = t0 + 100

    th_send.join().unwrap();
}

// Channel closed due to the receiver being dropped.
#[test]
fn send_after_close() {
    let (mut s, r) = channel(100);

    block_on(s.send(3)).unwrap();
    block_on(s.send(7)).unwrap();

    drop(r);

    assert_eq!(block_on(s.send(13)), Err(SendError(13)));
    assert_eq!(s.try_send(42), Err(TrySendError::Closed(42)));
}

// Channel closed due to the receiver being dropped while a sender is blocked on
// a full channel.
#[cfg(not(miri))]
#[test]
fn blocked_send_after_close() {
    let (mut s1, r) = channel(2);
    let mut s2 = s1.clone();

    block_on(s1.send(3)).unwrap();
    block_on(s1.send(7)).unwrap();

    let th_send1 = thread::spawn(move || {
        assert_eq!(block_on(s1.send(13)), Err(SendError(13))); // blocked from t0 to t0 + 100
    });
    let th_send2 = thread::spawn(move || {
        assert_eq!(block_on(s2.send(42)), Err(SendError(42))); // blocked from t0 to t0 + 100
    });

    sleep(100);
    drop(r); // t = t0 + 100

    th_send1.join().unwrap();
    th_send2.join().unwrap();
}

// Channel closed due to the senders being dropped.
#[test]
fn recv_after_close() {
    let (mut s1, mut r) = channel(100);
    let mut s2 = s1.clone();

    block_on(s1.send(3)).unwrap();
    block_on(s1.send(7)).unwrap();
    block_on(s2.send(13)).unwrap();

    drop(s1);
    drop(s2);

    assert_eq!(block_on(r.recv()), Ok(3));
    assert_eq!(block_on(r.recv()), Ok(7));
    assert_eq!(block_on(r.recv()), Ok(13));
    assert_eq!(block_on(r.recv()), Err(RecvError));
    assert_eq!(r.try_recv(), Err(TryRecvError::Closed));
}

// Channel closed due to the senders being dropped while the receiver is blocked
// on an empty channel.
#[cfg(not(miri))]
#[test]
fn blocked_recv_after_close() {
    let (mut s1, mut r) = channel(100);
    let mut s2 = s1.clone();

    block_on(s1.send(3)).unwrap();
    block_on(s1.send(7)).unwrap();
    block_on(s2.send(13)).unwrap();

    let th_recv = thread::spawn(move || {
        assert_eq!(block_on(r.recv()), Ok(3));
        assert_eq!(block_on(r.recv()), Ok(7));
        assert_eq!(block_on(r.recv()), Ok(13));
        assert_eq!(block_on(r.recv()), Err(RecvError)); // blocked from t0 to t0 + 100
        assert_eq!(r.try_recv(), Err(TryRecvError::Closed));
    });

    sleep(100);
    drop(s1);
    drop(s2);

    th_recv.join().unwrap();
}

// Block two senders on a full channel, cancel the first sending operation and
// receive a message to unblock the second sender.
#[cfg(not(miri))]
#[test]
fn cancel_async_send() {
    let (mut s1, mut r) = channel(2);
    let mut s2 = s1.clone();

    // Fill the channel and block a sender, then cancel the sending operation at
    // t0 + 300.
    let th_send1 = thread::spawn(move || {
        block_on(s1.send(3)).unwrap();
        block_on(s1.send(7)).unwrap();
        assert_eq!(poll_once_and_keep_alive(s1.send(13), 300), Poll::Pending); // cancel at t0 + 300
    });

    // Block a second sender from t0 + 100, expect it to get re-scheduled when the
    // sending operation of the first blocked sender is cancelled.
    let th_send2 = thread::spawn(move || {
        sleep(100);
        block_on(s2.send(42)).unwrap(); // blocked from t0 + 100 to t0 + 300
    });

    // Receive a message at t0 + 200 to free one channel slot; receive the
    // remaining messages at t0 + 400.
    let th_recv = thread::spawn(move || {
        sleep(200);
        assert_eq!(block_on(r.recv()), Ok(3)); // t = t0 + 200
        sleep(200);
        assert_eq!(r.try_recv(), Ok(7)); // t = t0 + 400
        assert_eq!(r.try_recv(), Ok(42)); // t = t0 + 400
    });

    th_send1.join().unwrap();
    th_send2.join().unwrap();
    th_recv.join().unwrap();
}

// Block two senders on a full channel, stop polling the first sender and
// receive two messages to unblock the second sender.
#[cfg(not(miri))]
#[test]
fn forget_async_send() {
    let (mut s1, mut r) = channel(2);
    let mut s2 = s1.clone();

    // Fill the channel and block a sender, then stop polling it for a long
    // time.
    let th_send1 = thread::spawn(move || {
        block_on(s1.send(3)).unwrap();
        block_on(s1.send(7)).unwrap();
        assert_eq!(poll_once_and_keep_alive(s1.send(13), 500), Poll::Pending);
    });

    // Block a second sender from t0 + 100, expect it to get re-scheduled when the
    // second message is received.
    let th_send2 = thread::spawn(move || {
        sleep(100);
        block_on(s2.send(42)).unwrap(); // blocked from t0 + 100 to t0 + 200
    });

    // Receive two message at t0 + 200 to free both channel slots; receive one
    // more message at t0 + 300 to check that the second sender got
    // re-scheduled.
    let th_recv = thread::spawn(move || {
        sleep(200);
        assert_eq!(block_on(r.recv()), Ok(3)); // t = t0 + 200
        assert_eq!(block_on(r.recv()), Ok(7)); // t = t0 + 200
        sleep(100);
        assert_eq!(r.try_recv(), Ok(42)); // t = t0 + 300
    });

    th_send1.join().unwrap();
    th_send2.join().unwrap();
    th_recv.join().unwrap();
}

// SPSC stress test.
#[test]
fn spsc_stress() {
    const CAPACITY: usize = 3;
    const COUNT: usize = if cfg!(miri) { 50 } else { 1_000_000 };

    let (mut s, mut r) = channel(CAPACITY);

    let th_send = thread::spawn(move || {
        block_on(async {
            for i in 0..COUNT {
                s.send(i).await.unwrap();
            }
        });
    });
    let th_recv = thread::spawn(move || {
        block_on(async {
            for i in 0..COUNT {
                assert_eq!(r.recv().await, Ok(i));
            }
        });

        assert!(r.try_recv().is_err());
    });

    th_send.join().unwrap();
    th_recv.join().unwrap();
}

// MPSC stress test.
#[test]
fn mpsc_stress() {
    const CAPACITY: usize = 3;
    const COUNT: usize = if cfg!(miri) { 50 } else { 1_000_000 };
    const THREADS: usize = 4;

    let (s, mut r) = channel(CAPACITY);

    let th_send = (0..THREADS).map(|_| {
        let mut s = s.clone();

        thread::spawn(move || {
            block_on(async {
                for i in 0..COUNT {
                    s.send(i).await.unwrap();
                }
            });
        })
    });
    let th_recv = thread::spawn(move || {
        let mut stats = Vec::new();
        stats.resize(COUNT, 0);

        block_on(async {
            for _ in 0..COUNT * THREADS {
                let i = r.recv().await.unwrap();
                stats[i] += 1;
            }
        });

        assert!(r.try_recv().is_err());

        for s in stats {
            assert_eq!(s, THREADS);
        }
    });

    for th in th_send {
        th.join().unwrap()
    }
    th_recv.join().unwrap();
}
