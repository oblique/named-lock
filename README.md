# named-lock

[![license][license badge]][license]
[![crates.io][crate badge]][crate]
[![docs][docs badge]][docs]

This crate provides a simple and cross-platform implementation of named locks.
You can use this to lock sections between processes.

## Example

```rust
use named_lock::NamedLock;
use named_lock::Result;

fn main() -> Result<()> {
    let lock = NamedLock::create("foobar")?;
    let _guard = lock.lock()?;

    // Do something...

    Ok(())
}
```

## Implementation

On UNIX this is implemented by using files and [`flock`]. The path of the
created lock file will be `$TMPDIR/<name>.lock`, or `/tmp/<name>.lock` if
`TMPDIR` environment variable is not set.

On Windows this is implemented by creating named mutex with [`CreateMutexW`].


[license]: LICENSE
[license badge]: https://img.shields.io/github/license/oblique/named-lock
[crate]: https://crates.io/crates/named-lock
[crate badge]: https://img.shields.io/crates/v/named-lock
[docs]: https://docs.rs/named-lock
[docs badge]: https://docs.rs/named-lock/badge.svg

[`flock`]: https://linux.die.net/man/2/flock
[`CreateMutexW`]: https://docs.microsoft.com/en-us/windows/win32/api/synchapi/nf-synchapi-createmutexw
