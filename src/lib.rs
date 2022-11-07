//! A very fast asynchronous, multi-producer, single-consumer (MPSC) bounded
//! channel.
//!
//! This is a no-frills `async` bounded MPSC channel which only claim to fame is
//! to be extremely fast, without taking any shortcuts on correctness
//! and implementation quality.
//!
//! # Disconnection
//!
//! The channel is disconnected automatically once all [`Sender`]s are dropped
//! or once the [`Receiver`] is dropped. It can also be disconnected manually by
//! any `Sender` or `Receiver` handle.
//!
//! Disconnection is signaled by the `Result` of the sending or receiving
//! operations. Once a channel is disconnected, all attempts to send a message
//! will return an error. However, the receiver will still be able to receive
//! messages already in the channel and will only get a disconnection error once
//! all messages have been received.
//!
//! # Example
//!
//! ```
//! use tachyonix;
//! use futures_executor::{block_on, ThreadPool};
//!
//! let pool = ThreadPool::new().unwrap();
//!
//! let (s, mut r) = tachyonix::channel(3);
//!
//! block_on( async move {
//!     pool.spawn_ok( async move {
//!         assert_eq!(s.send("Hello").await, Ok(()));
//!     });
//!     
//!     assert_eq!(r.recv().await, Ok("Hello"));
//! });
//! # std::thread::sleep(std::time::Duration::from_millis(100)); // MIRI bug workaround
//! ```
//!
#![warn(missing_docs, missing_debug_implementations, unreachable_pub)]

mod event;
mod loom_exports;
mod queue;

use std::error;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{self, AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::Context;
use std::task::Poll;

use diatomic_waker::primitives::DiatomicWaker;
use futures_core::Stream;

use crate::event::Event;
use crate::queue::{PopError, PushError, Queue};

/// Shared channel data.
struct Inner<T> {
    /// Non-blocking internal queue.
    queue: Queue<T>,
    /// Signalling primitive used to notify the receiver.
    receiver_signal: DiatomicWaker,
    /// Signalling primitive used to notify one or several senders.
    sender_signal: Event,
    /// Current count of live senders.
    sender_count: AtomicUsize,
}

impl<T> Inner<T> {
    fn new(capacity: usize, sender_count: usize) -> Self {
        Self {
            queue: Queue::new(capacity),
            receiver_signal: DiatomicWaker::new(),
            sender_signal: Event::new(),
            sender_count: AtomicUsize::new(sender_count),
        }
    }
}

/// The sending side of a channel.
///
/// Multiple [`Sender`]s can be created via cloning.
pub struct Sender<T> {
    /// Shared data.
    inner: Arc<Inner<T>>,
}

impl<T> Sender<T> {
    /// Attempts to send a message immediately.
    pub fn try_send(&self, message: T) -> Result<(), TrySendError<T>> {
        match self.inner.queue.push(message) {
            Ok(()) => {
                self.inner.receiver_signal.notify();
                Ok(())
            }
            Err(PushError::Full(v)) => Err(TrySendError::Full(v)),
            Err(PushError::Closed(v)) => Err(TrySendError::Closed(v)),
        }
    }

    /// Sends a message asynchronously, if necessary waiting until enough
    /// capacity becomes available.
    pub async fn send(&self, message: T) -> Result<(), SendError<T>> {
        let mut message = Some(message);

        self.inner
            .sender_signal
            .wait_until(|| {
                match self.inner.queue.push(message.take().unwrap()) {
                    Ok(()) => Some(()),
                    Err(PushError::Full(m)) => {
                        // Recycle the message.
                        message = Some(m);

                        None
                    }
                    Err(PushError::Closed(m)) => {
                        // Keep the message so it can be returned in the error
                        // field.
                        message = Some(m);

                        Some(())
                    }
                }
            })
            .await;

        match message {
            Some(m) => Err(SendError(m)),
            None => {
                self.inner.receiver_signal.notify();

                Ok(())
            }
        }
    }

    /// Closes the queue.
    ///
    /// This prevents any further messages from being sent on the channel.
    /// Messages that were already sent can still be received.
    pub fn close(&self) {
        self.inner.queue.close();

        // Notify the receiver and all blocked senders that the channel is
        // closed.
        self.inner.receiver_signal.notify();
        self.inner.sender_signal.notify(usize::MAX);
    }

    /// Checks if the channel is closed.
    ///
    /// This can happen either because the [`Receiver`] was dropped or because
    /// one of the [`Sender::close`] or [`Receiver::close`] method was called.
    pub fn is_closed(&self) -> bool {
        self.inner.queue.is_closed()
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        // Increase the sender reference count.
        //
        // Ordering: Relaxed ordering is sufficient here for the same reason it
        // is sufficient for an `Arc` reference count increment: synchronization
        // is only necessary when decrementing the counter since all what is
        // needed is to ensure that all operations until the drop handler is
        // called are visible once the reference count drops to 0.
        self.inner.sender_count.fetch_add(1, Ordering::Relaxed);

        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        // Decrease the sender reference count.
        //
        // Ordering: Release ordering is necessary for the same reason it is
        // necessary for an `Arc` reference count decrement: it ensures that all
        // operations performed by this sender before it was dropped will be
        // visible once the sender count drops to 0.
        if self.inner.sender_count.fetch_sub(1, Ordering::Release) == 1
            && !self.inner.queue.is_closed()
        {
            // Make sure that the notified receiver sees all operations
            // performed by all dropped senders.
            //
            // Ordering: Acquire is necessary to synchronize with the Release
            // decrement operations. Note that the fence synchronizes with _all_
            // decrement operations since the chain of counter decrements forms
            // a Release sequence.
            atomic::fence(Ordering::Acquire);

            self.inner.queue.close();

            // Notify the receiver that the channel is closed.
            self.inner.receiver_signal.notify();
        }
    }
}

impl<T> fmt::Debug for Sender<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Sender").finish_non_exhaustive()
    }
}

/// The receiving side of a channel.
///
/// The receiver can only be called from a single thread.
pub struct Receiver<T> {
    /// Shared data.
    inner: Arc<Inner<T>>,
}

impl<T> Receiver<T> {
    /// Attempts to receive a message immediately.
    pub fn try_recv(&mut self) -> Result<T, TryRecvError> {
        // Safety: `Queue::pop` cannot be used concurrently from multiple
        // threads since `Receiver` does not implement `Clone` and requires
        // exclusive ownership.
        match unsafe { self.inner.queue.pop() } {
            Ok(message) => {
                self.inner.sender_signal.notify(1);
                Ok(message)
            }
            Err(PopError::Empty) => Err(TryRecvError::Empty),
            Err(PopError::Closed) => Err(TryRecvError::Closed),
        }
    }

    /// Receives a message asynchronously, if necessary waiting until one
    /// becomes available.
    pub async fn recv(&mut self) -> Result<T, RecvError> {
        // We could of course return the future directly from a plain method,
        // but the `async` signature makes the intent more explicit.
        RecvFuture { receiver: self }.await
    }

    /// Closes the queue.
    ///
    /// This prevents any further messages from being sent on the channel.
    /// Messages that were already sent can still be received, however, which is
    /// why a call to this method should typically be followed by a loop
    /// receiving all remaining messages.
    ///
    /// For this reason, no counterpart to [`Sender::is_closed`] is exposed by
    /// the receiver as such method could easily be misused and lead to lost
    /// messages. Instead, messages should be received until a [`RecvError`] or
    /// [`TryRecvError::Closed`] error is returned.
    pub fn close(&self) {
        if !self.inner.queue.is_closed() {
            self.inner.queue.close();

            // Notify all blocked senders that the channel is closed.
            self.inner.sender_signal.notify(usize::MAX);
        }
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.inner.queue.close();

        // Notify all blocked senders that the channel is closed.
        self.inner.sender_signal.notify(usize::MAX);
    }
}

impl<T> fmt::Debug for Receiver<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Receiver").finish_non_exhaustive()
    }
}

impl<T> Stream for Receiver<T> {
    type Item = T;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // Safety: `Queue::pop`, `DiatomicWaker::register` and
        // `DiatomicWaker::unregister` cannot be used concurrently from multiple
        // threads since `Receiver` does not implement `Clone` and requires
        // exclusive ownership.
        unsafe {
            // Happy path: try to pop a message without registering the waker.
            match self.inner.queue.pop() {
                Ok(message) => {
                    // Signal to one awaiting sender that one slot was freed.
                    self.inner.sender_signal.notify(1);

                    return Poll::Ready(Some(message));
                }
                Err(PopError::Closed) => {
                    return Poll::Ready(None);
                }
                Err(PopError::Empty) => {}
            }

            // Slow path: we must register the waker to be notified when the
            // queue is populated again. It is thereafter necessary to check
            // again the predicate in case we raced with a sender.
            self.inner.receiver_signal.register(cx.waker());

            match self.inner.queue.pop() {
                Ok(message) => {
                    // Cancel the request for notification.
                    self.inner.receiver_signal.unregister();

                    // Signal to one awaiting sender that one slot was freed.
                    self.inner.sender_signal.notify(1);

                    Poll::Ready(Some(message))
                }
                Err(PopError::Closed) => {
                    // Cancel the request for notification.
                    self.inner.receiver_signal.unregister();

                    Poll::Ready(None)
                }
                Err(PopError::Empty) => Poll::Pending,
            }
        }
    }
}

/// The future returned by the `Receiver::recv` method.
///
/// This is just a thin wrapper over the `Stream::poll_next` implementation.
struct RecvFuture<'a, T> {
    receiver: &'a mut Receiver<T>,
}

impl<'a, T> Future for RecvFuture<'a, T> {
    type Output = Result<T, RecvError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match Pin::new(&mut self.receiver).poll_next(cx) {
            Poll::Ready(Some(v)) => Poll::Ready(Ok(v)),
            Poll::Ready(None) => Poll::Ready(Err(RecvError)),
            Poll::Pending => Poll::Pending,
        }
    }
}

/// Creates a new channel, returning the sending and receiving sides.
///
/// # Panic
///
/// The function will panic if the requested capacity is 0 or if it is greater
/// than `usize::MAX/2 + 1`.
pub fn channel<T>(capacity: usize) -> (Sender<T>, Receiver<T>) {
    let inner = Arc::new(Inner::new(capacity, 1));

    let sender = Sender {
        inner: inner.clone(),
    };
    let receiver = Receiver { inner };

    (sender, receiver)
}

/// An error returned when an attempt to send a message synchronously is
/// unsuccessful.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrySendError<T> {
    /// The queue is full.
    Full(T),
    /// The receiver has been dropped.
    Closed(T),
}

impl<T: fmt::Debug> error::Error for TrySendError<T> {}

impl<T> fmt::Display for TrySendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TrySendError::Full(_) => "Full(..)".fmt(f),
            TrySendError::Closed(_) => "Closed(..)".fmt(f),
        }
    }
}

/// An error returned when an attempt to receive a message synchronously is
/// unsuccessful.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TryRecvError {
    /// The queue is empty.
    Empty,
    /// All senders have been dropped.
    Closed,
}

impl error::Error for TryRecvError {}

impl fmt::Display for TryRecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TryRecvError::Empty => "receiving from an empty channel".fmt(f),
            TryRecvError::Closed => "receiving from a closed channel".fmt(f),
        }
    }
}

/// An error returned when an attempt to send a message asynchronously is
/// unsuccessful.
#[derive(Clone, Copy, Eq, PartialEq)]
pub struct SendError<T>(pub T);

impl<T: fmt::Debug> error::Error for SendError<T> {}

impl<T> fmt::Debug for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SendError").finish_non_exhaustive()
    }
}

impl<T> fmt::Display for SendError<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "sending into a closed channel".fmt(f)
    }
}

/// An error returned when an attempt to receive a message asynchronously is
/// unsuccessful.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RecvError;

impl error::Error for RecvError {}

impl fmt::Display for RecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        "receiving from a closed channel".fmt(f)
    }
}
