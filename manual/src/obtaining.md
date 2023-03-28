# Obtaining fortISSimO

There are two ways of obtaining fortISSimO.
[Using a development version](#using-a-development-version) may be somewhat less reliable than a release, but gives you access to some features earlier, and any testing on those is *highly* appreciated.

## Grabbing a release

First, pick one of [the releases](https://github.com/ISSOtm/fortISSimO/releases).
Then, either grab one of the pre-built bundles, or either of the "source code" downloads if you want to compile teNOR yourself or no pre-built is available for your setup.

### Pre-built bundle

Since the bundle includes a pre-built binary of teNOR, you must grab the one corresponding to your computer's architecture:

- Windows: `x86_64-pc-windows-msvc`[^esoteric_arch]
- Linux: `x86_64-unknown-linux-gnu`[^esoteric_arch]
- macOS: `aarch64-apple-darwin` if on "Apple Silicon", `x86_64-apple-darwin` otherwise[^esoteric_arch]<sup>, </sup>[^ppc_mac].

You can then use all of the provided files any way you like.
To update fortISSimO, simply overwrite all of the files; delete any files not in the new bundle.

[^esoteric_arch]: If you don't have a 64-bit Intel processor, you will have to go the "[Source code](#source-code)" route instead.

### Source code

fortISSimO itself can be used just the same, but you will need to compile teNOR yourself.
teNOR is written in Rust, so you must [install it](https://www.rust-lang.org/tools/install).
Then, you can build teNOR by running `cargo build --release` inside of the `teNOR/` directory; the resulting binary will be in the `target/release/` directory.

## Using a development version

First, [clone the repository](https://docs.github.com/en/repositories/creating-and-managing-repositories/cloning-a-repository); then, follow the "[Source code](#source-code)" instructions.
Grabbing a ZIP is also fine.

That said, if you want to integrate fortISSimO in a project that already uses Git, consider [using a submodule](https://git-scm.com/docs/gitsubmodules) to make upgrading fortISSimO easier.

[^ppc_mac]: If you have a PowerPC Mac, you have my respect (*and* no pre-built binaries :D)
