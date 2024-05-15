# 0.3.0 (2024-05-15)

* Make it possible to specify a deadline when sending or receiving ([#6]).
* Increase the MSRV to work around breakage introduced by the new `--check-cfg`
  being enabled by default.

*Note*: there are no API-breaking changes, the minor version was only increased
due to the new MSRV.

[#6]: https://github.com/asynchronics/tachyonix/pull/6

# 0.2.1 (2023-07-16)

- Use internally the newly spun-off `async-event` crate instead of the `event`
  module.
- Use `crossbeam-utils` instead of deprecated `cache-padded` crate.

# 0.2.0 (2022-10-30)

- Improve performance by always allocating new notifiers for blocked senders;
  this also makes it now possible take `self` by reference in senders ([#3]).
- Fix soundness issue when sender is forgotten ([#2]).

[#2]: https://github.com/asynchronics/tachyonix/pull/2
[#3]: https://github.com/asynchronics/tachyonix/pull/3


# 0.1.1 (2022-10-16)

- Support Rust 1.56.
- Move benchmark to separate repo.


# 0.1.0 (2022-10-12)

Initial release
