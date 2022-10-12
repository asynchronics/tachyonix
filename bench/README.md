# The tachyonix benchmark

This is a benchmarking suite for `async` MPSC message passing.


## Channels, runtimes and benchmarks

At the moment, it includes the following MPMC/MPSC channels:
- [tachyonix](https://github.com/asynchronics/tachyonix)
- [async-channel](https://github.com/smol-rs/async-channel)
- [flume](https://github.com/zesterer/flume)
- [postage::mpsc](https://github.com/austinjones/postage-rs)
- [tokio::mpsc](https://github.com/tokio-rs/tokio)

It is also possible to select one of the following runtimes:
- [asynchronix](https://github.com/asynchronics/asynchronix)
- [tokio](https://github.com/tokio-rs/tokio)
- [async-std](https://github.com/async-rs/async-std)
- [smolscale](https://github.com/geph-official/smolscale)

There are currently 2 parametric benchmarks:
- *pinball*: a fully connected graph where messages (the "balls") perform a
  random walk between nodes (the "pins"),
- *funnel*: a set of receivers sending messages to a unique receiver in a tight
  loop.


## Usage

For help, type:

```
$ bench -h
```

To see all benchmarks for all channels, type:

```
$ bench -l
```

To run the *pinball* benchmark for all channels using Tokio, type:

```
$ bench pinball
```

To run all benchmarks for `tachonix` using Tokio, type:

```
$ bench tachyonix
```

To run only the *funnel* benchmark for `flume` using Tokio, type:

```
$ bench funnel-flume
```

To run all benchmarks for `async-channel` with Asynchronix instead of Tokio, type:

```
$ bench async_channel -e asynchronix
```



## Benchmark details, License, etc

Please see the main [Readme](https://github.com/asynchronics/tachyonix).