# teNOR

> teNOR stands for "**t**racker-less **e**xporter with **N**otably **O**ptimised **R**esults"[^backronym].

teNOR is a command-line program that converts `.uge` files saved by hUGETracker into `.asm` files.
It can be considered an alternative to `uge2source`, tailored to fortISSimO.

Before talking about how to use it, here is teNOR's built-in short help text:

```console
$ ./teNOR -h
{{#include ./teNOR.help}}
```

[^backronym]: This is _totally_ not a [backronym](https://en.wikipedia.org/wiki/Backronym). What? ...You don't believe me?

## Usage

teNOR is a command-line program; to use it, you should at least know how to open a terminal, change directories in it, and execute programs.
(An alternative might be to use `.bat` files on Windows / `.sh` files anywhere else.)

I believe the core usage should be simple enough, so let's talk about some of the options.

### Song descriptor

The "song descriptor" is the label that will have to be passed to [`hUGE_SelectSong`](./integration.md) later.
Since it is a label, it must be a valid [RGBASM symbol](https://rgbds.gbdev.io/docs/rgbasm.5/#SYMBOLS) name (regex: `[A-Za-z_][A-Za-z0-9_#@$]*`), and since it will be exported, it must be **unique across the entire program**.

### Stats

teNOR tries to optimise the exported data to take less space.
When it's done running, it prints statistics about how much space the optimisations saved; this was originally done to check if they were worth the trouble, and then kept because honestly, why not?

If you don't care about the stats, pass the `-q`/`--quiet` option to silence them.

> Note that the reported savings are **not** the difference with the size of an equivalent hUGEDriver export, due to other, more fundamental format differences.
> Unoptimised fortISSimO exports _should_ be smaller than hUGEDriver exports; how much varies from version to version.

## Output file

teNOR aims to produce output files that are easy to understand and nicely formatted.
If you want to read the generated file, go ahead!
