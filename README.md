# tachyonix

An asynchronous, multi-producer, single-consumer (MPSC) bounded channel
that operates at [tachyonic][tachyon] speeds.

This library is an offshot of [Asynchronix][asynchronix], an ongoing effort at a
high performance asynchronous computation framework for system simulation.

No laws of physics were broken in the making of this library.

[![Cargo](https://img.shields.io/crates/v/tachyonix.svg)](https://crates.io/crates/tachyonix)
[![Documentation](https://docs.rs/tachyonix/badge.svg)](https://docs.rs/tachyonix)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](https://github.com/asynchronics/tachyonix#license)

[tachyon]: https://en.wikipedia.org/wiki/Tachyon

[asynchronix]: https://github.com/asynchronics/asynchronix

## Overview

This is a no-frills `async` channel which only claim to fame is to be extremely
fast (see [benchmarks](#benchmarks)), without taking any shortcuts on
correctness and implementation quality. Its performance mainly results from its
focus on the MPSC use-case and from a number of careful optimizations, among
which:

- aggressively optimized notification primitives for full-queue and
  empty-queue events (the latter is courtesy of
  [diatomic-waker][diatomic-waker], a fast, spinlock-free alternative to
  `atomic-waker`),
- no allocation once the senders are created, even for blocked sender/receiver
  notifications,
- no spinlocks[^spinlocks] and conservative use of Mutexes (only used for
  blocked senders),
- underlying queue optimized for the single receiver use-case.

[diatomic-waker]: https://github.com/asynchronics/diatomic-waker

[^spinlocks]: some MPMC channels use spinlocks, such as `crossbeam-channel` and
    `async-channel` where they are required to linearize the underlying queue.


## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
tachyonix = "0.1.1"
```


## Example

```rust
use tachyonix;
use futures_executor::{block_on, ThreadPool};

let pool = ThreadPool::new().unwrap();

let (mut s, mut r) = tachyonix::channel(3);

block_on( async move {
    pool.spawn_ok( async move {
        assert_eq!(s.send("Hello").await, Ok(()));
    });
    
    assert_eq!(r.recv().await, Ok("Hello"));
});
```


## Limitations

The original raison d'être of this library was to provide a less idiosyncratic
sibling to the channels developed for [Asynchronix][asynchronix] that could be
easily benchmarked against other channel implementations. The experiment turned
out better than anticipated so a slightly more fleshed out version was released
for public consumption in the hope that others may find it useful. However, its
API surface is intentionally kept small and it does not aspire to become much
more than it is today.

Note also that, just like the bounded channel of the `futures` crate but unlike
most other channels, sending requires mutable access to a `Sender`.

[sink]: https://docs.rs/futures/latest/futures/sink/trait.Sink.html

[stream]: https://docs.rs/futures/latest/futures/stream/trait.Stream.html

[channel_capacity]:
    https://github.com/rust-lang/futures-rs/pull/984#issuecomment-383792953


## Safety

Despite the focus on performance, implementation quality and correctness are the
highest priority. The library comes with a decent battery of tests, in
particular for all low-level (unsafe) concurrency primitives which are
extensively tested with [Loom][loom], complemented with MIRI for integrations
tests. As amazing as they are, however, Loom and MIRI cannot formally prove the
absence of data races so soundness issues _are_ possible. You should therefore
exercise caution before using it in mission-critical software until it receives
more testing in the wild.

[loom]: https://github.com/tokio-rs/loom


## Benchmarks

### Benchmarks overview

A custom [benchmarking suite][bench] was implemented that can test a number of
popular MPSC and MPMS channels with several executors (Tokio, async-std,
smolscale and Asynchronix).

It contains at the moment 2 benchmarks:
- *pinball*: an upgraded version of the classical pin-pong benchmark where
  messages ("balls") perform a random walk between 13 vertices ("pins") of a
  fully connected graph; it is parametrized by the total number of balls within
  the graph,
- *funnel*: the most common MPSC benchmark where messages are sent in a tight
  loop from 13 senders to a unique receiver; it is parametrized by the channel
  capacity.

Each benchmark executes 61 instances of an elementary bench rig, which ensures
that all executor threads are busy at nearly all times. The *pinball* benchmark
is a relatively good proxy for performance in situations where channel receivers
are often starved but senders are never blocked (i.e. the channel capacity is
always sufficient). The *funnel* benchmark is less objective and more difficult
to interpret as it is sensitive not only to the absolute speed of enqueue,
dequeue and notifications, but can also be affected by their relative speed.

More information about these benchmarks can be found in the [bench repo][bench].

[bench]: https://github.com/asynchronics/tachyobench/

### Benchmark results

The benchmarks were run on EC2 instances of comparable performance but different
micro-architectures (Intel Ice Lake, AMD Zen 3, ARM Graviton 2). The reported
performance is the mean number of messages per microsecond after averaging over
10 benchmark runs.

The reported results were obtained with Tokio, which in practice was found
significantly faster than either async-std or smolscale. Asynchronix is faster
yet, but probably less relevant as a baseline as it is not meant for
general-purpose `async` programming.

#### EC2 c6i.2xlarge

![Alt text](https://raw.githubusercontent.com/asynchronics/tachyobench/main/results/tokio_2022-11-10/c6i.2xlarge.png)

#### EC2 c6a.2xlarge

![Alt text](https://raw.githubusercontent.com/asynchronics/tachyobench/main/results/tokio_2022-11-10/c6a.2xlarge.png)

#### EC2 c6g.2xlarge

![Alt text](https://raw.githubusercontent.com/asynchronics/tachyobench/main/results/tokio_2022-11-10/c6g.2xlarge.png)


## License

This software is licensed under the [Apache License, Version
2.0](LICENSE-APACHE) or the [MIT license](LICENSE-MIT), at your option.


## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
