# Usage

> There is [a demo project](https://github.com/ISSOtm/fortISSimO-demo) for fortISSimO, that can serve as a reference, and as a quick way to preview your track.

Using fortISSimO has two parts to it: converting your songs, and playing them back.
The former is handled by teNOR, the latter is handled by fortISSimO itself.

**Neither `uge2source` nor hUGETracker's "Export to RGBDS asm..." function are suitable for fortISSimO!**
For this reason, it's quite advisable to put the `.uge` files directly in your source files, and run teNOR as part of your build process.

Since both teNOR and fortISSimO cooperate tightly together, **you must use compatible versions of both**!
If you don't, you *should* get an error telling you so.
teNOR and fortISSimO both follow [semantic versioning](https://semver.org), which here means that versions `x.y.z` and `x'.y'.z'` are compatible *if and only if* `x` and `x'` are equal.

Up next:
- [Exporting your songs](./teNOR.md)
- Playing your songs back:
  - [RGBDS projects](./rgbds.md)
  - [GBDK projects](./gbdk.md)
