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
fast (see [benchmarks](#benchmarks)). Its performance mainly results from its
focus on the MPSC use-case and from a number of careful optimizations, among
which:

- aggressively optimized notification primitives for full-queue and
  empty-queue events (the latter is courtesy of
  [diatomic-waker][diatomic-waker], a fast, spinlock-free alternative to
  `atomic-waker`),
- no allocation after the senders are created, even for blocked sender/receiver
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
tachyonix = "0.1.0"
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

Despite the focus on performance, implementation quality and correctness are a
very high priority. The library comes with a decent battery of tests, in
particular for all low-level (unsafe) concurrency primitives which are
extensively tested with [Loom][loom]. As amazing as it is, however, Loom cannot
formally prove the absence of data races so soundness issues _are_ possible. You
should therefore exercise caution before using it in mission-critical software
until it receives more testing in the wild.

[loom]: https://github.com/tokio-rs/loom


## Benchmarks

Benchmarking is hard. Async benchmarking even more so.

When benchmarking `async` code, a lot depends on the detailed implementation and
scheduling strategy of the executor. Performance variability is typically high,
in particular when the load is not sufficient to keep all worker threads busy. 

With this caveat, a custom [benchmarking suite][bench] was implemented that can
test not only several channels, but also several executors (Tokio, async-std,
smolscale and of course Asynchronix). It contains at the moment 2 benchmarks,
*pinball* and *funnel*. Each benchmark executes 61 instances of an elementary
bench rig, which ensures that the executor is fully loaded at all times.

Why the prime numbers everywhere? This is meant to avoid unwanted "mechanical
sympathy" with the number of executor threads.

[bench]: https://github.com/asynchronics/tachyonix/bench/

### Pinball

This benchmark is an upgraded version of the classical ping-pong benchmark. Its
main goal is to measure performance in situations where receivers are often
starved but sender are never blocked.

Each test rig consists of a complete graph (a.k.a. fully connected graph) which
edges are the channels. Each node forwards any message it receives to another
randomly chosen node. For this benchmark, each graph contains 13 nodes, each
node containing in turn 1 receiver and 12 senders (1 for each other node).
Importantly, each channel has enough capacity to never block on sending. The
benchmark concurrently runs 61 such rigs of 13 nodes.

The test is performed for various numbers of messages ("balls"), which are
initially fairly distributed across the graph. The messages then perform a
random walk between the nodes ("pins") until they have visited a pre-defined
amount of nodes.

### Funnel

This benchmark is ubiquitous and known by many names. It is often simply
referred to as the "MPSC benchmark". It consists of a single receiver connected
to many senders sending messages in a tight loop.

What this benchmark measures is very dependent on many details such as the
relative timing of enqueue, dequeue and notification operations. Unsurprisingly,
the standard deviation on the results is large compared to the pinball
benchmark. Corollary: this benchmark is neither very realistic nor very
objective and one should be cautious when interpreting the results.

In this particular implementation, each receiver is connected to 13 senders. The
benchmark runs 61 such rigs of 13 senders and 1 receiver concurrently.

The test is performed for various channel capacities. Note that unlike the other
channels, tokio's MPSC channel reserves 1 additional slot for each of the 13
senders on top of the nominal capacity.

### Results

The benchmarks were run on EC2 instances of comparable performance but different
micro-architectures (Intel Ice Lake, AMD Zen 3, ARM Graviton 2). The reported
performance is the mean number of messages per microsecond after averaging over
10 benchmark runs.

The reported results were obtained with Tokio, which in practice was found
significantly faster than either async-std or smolscale. Asynchronix is faster
yet, but would not constitute a pragmatic baseline as it is not meant for
general-purpose `async` programming.

#### EC2 c6i.2xlarge

![Alt text](/bench/results/tokio_2022-11-10/c6i.2xlarge.png)

#### EC2 c6a.2xlarge

![Alt text](/bench/results/tokio_2022-11-10/c6a.2xlarge.png)

#### EC2 c6g.2xlarge

![Alt text](/bench/results/tokio_2022-11-10/c6g.2xlarge.png)


## License

This software is licensed under the [Apache License, Version
2.0](LICENSE-APACHE) or the [MIT license](LICENSE-MIT), at your option.

Note, however, that the distribution of the test suite and benchmark is
constrained by its dependencies which may be subject to other licenses.


## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
