# mcdownload

## Installation

### Compiled binaries

Binaries for Linux (amd64 and aarch64) and Windows (amd64) are available as [Actions artifacts][actions]
or, if you're not logged in, from [nightly.link][nightly]. If you're not sure which one to use,
try `linux-nightly-release` for Linux and `win-msvc-release` for Windows.

### From source

```sh
cargo install --git https://github.com/paenis/mcdownload
```

Requires a system OpenSSL installation, or, if building with `--features vendored-openssl`, a C compiler, perl, perl-core, and make. See [openssl] for more information.

[openssl]: https://docs.rs/openssl/latest/openssl/#building
[actions]: https://github.com/paenis/mcdownload/actions?query=is%3Asuccess
[nightly]: https://nightly.link/paenis/mcdownload/workflows/test/main

## Todo

- [ ] types/meta
  - [ ] `Settings` struct
    - [ ] global default java flags (maybe)
  - [x] manifest [#3][pull-3]
- [ ] main
  - [ ] alternative outputs (JSON/debug/etc.) for info/list commands
  - [ ] `-i`/`--installed` filter for list command
    - [x] likely depends on [#3][pull-3]
  - [x] `tracing_error` for `eyre`, maybe
  - [ ] third-party servers (fabric, forge, etc.) [#1][pull-1]
- [ ] types/version
  - [ ] fabric meta (?)

[pull-1]: https://github.com/paenis/mcdownload/pull/1
[pull-3]: https://github.com/paenis/mcdownload/pull/3

---

By using this software, you agree to the [Minecraft EULA][eula].

[eula]: https://www.minecraft.net/en-us/eula
