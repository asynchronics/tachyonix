# ?.?.? (????-??-??)

- Improve performance by always allocating new notifiers for blocked senders;
  this also makes it now possible take `self` by reference in senders.
- Fix soundness issue when sender is forgotten ([#2])

[#2]: https://github.com/asynchronics/tachyonix/pull/2


# 0.1.1 (2022-10-16)

- Support Rust 1.56.
- Move benchmark to separate repo.


# 0.1.0 (2022-10-12)

Initial release
