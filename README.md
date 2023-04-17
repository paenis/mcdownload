# mcdownload

## Installation

### Compiled binaries

Binaries for Linux (amd64 and aarch64) and Windows (amd64) are available as [Actions artifacts][actions]
or, if you're not logged in, from [nightly.link][nightly]. If you're not sure which one to use,
try `linux-nightly-release` for Linux and `win-msvc-release` for Windows.

### From source

<!-- TODO: this (i think) installs to .cargo/bin, so i should probably change the folder structure to not clobber anything -->
```sh
cargo install --git https://github.com/paenis/mcdownload
```

[actions]: https://github.com/paenis/mcdownload/actions?query=is%3Asuccess
[nightly]: https://nightly.link/paenis/mcdownload/workflows/test/main

## Todo

- [ ] types/meta
  - [ ] `Settings` struct
    - [ ] global default java flags (maybe)
- [ ] main
  - [ ] alternative outputs (JSON/debug/etc.) for info/list commands
  - [ ] `-i`/`--installed` filter for list command
  - [ ] `tracing_error` for `eyre`, maybe
  - [ ] third-party servers (fabric, forge, etc.) [#1](https://github.com/paenis/mcdownload/pull/1)
- [ ] types/version
  - [ ] fabric meta (?)
