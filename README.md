# interrupt-mutex

[![Crates.io](https://img.shields.io/crates/v/interrupt-mutex)](https://crates.io/crates/interrupt-mutex)
[![docs.rs](https://img.shields.io/docsrs/interrupt-mutex)](https://docs.rs/interrupt-mutex)
[![CI](https://github.com/mkroening/interrupt-mutex/actions/workflows/ci.yml/badge.svg)](https://github.com/mkroening/interrupt-mutex/actions/workflows/ci.yml)

A mutex for sharing data with interrupt handlers or signal handlers.

```rust
// Make a mutex of your choice into an `InterruptMutex`.
type InterruptMutex<T> = interrupt_mutex::InterruptMutex<parking_lot::RawMutex, T>;

static X: InterruptMutex<Vec<i32>> = InterruptMutex::new(Vec::new());

fn interrupt_handler() {
    X.lock().push(1);
}

let v = X.lock();
// Raise an interrupt
raise_interrupt();
assert_eq!(*v, vec![]);
drop(v);

// The interrupt handler runs

let v = X.lock();
assert_eq!(*v, vec![1]);
drop(v);
```

For API documentation, see the [docs].

[docs]: https://docs.rs/interrupt-mutex

## License

Licensed under either of

 * Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license
   ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
