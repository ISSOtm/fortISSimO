# Song format

This describes the format that songs are stored in fortISSimO at the binary level.
The format of files exported by [teNOR] is kind of irrelevant, and the format of hUGETracker's `.uge` files is documented elsewhere.

> Last updated as of commit [`7fb8329`](https://github.com/ISSOtm/fortISSimO/tree/7fb83298d9aa0296fa075d7584a13abdd41b7d06).
> This is susceptible to having changed since then; see [the changes since then](https://github.com/ISSOtm/fortISSimO/compare/7fb83298d9aa0296fa075d7584a13abdd41b7d06...master), particularly files `teNOR/export.rs` and `include/fortISSimO.inc`.

ℹ️ For forward compatibility's sake, it is unwise to assume that components will always be in a certain order unless otherwise specified.

For example, currently, [teNOR] emits duty instruments immediately after the "row pool", and right before the wave instruments; this should be considered an unstable implementation detail.
However, all fields of the song header will remain in the specified order (until the next major release of fO, anyway).

Unless specified:

- there is **no padding** between any of the structures' fields,
- all multi-byte values are stored in little-endian format (low byte first).

## Header

1. **BYTE** — How many ticks each row lasts for.
1. **BYTE** — The maximum value `wOrderIdx` can take; inclusive. Also specifies the size of the pattern pointer arrays, below.
1. **POINTER** — To the array of duty instruments.
1. **POINTER** — To the array of wave instruments.
1. **POINTER** — To the array of noise instruments.
1. **POINTER** — To the song's routine.
1. **POINTER** — To the array of waves.
1. **BYTE** — High byte of the pointer to the "main" patterns' cell catalog ([see below][catalog]).
1. **BYTE** — High byte of the pointer to the subpatterns' cell catalog ([see below][catalog]).
1. _For each channel, from `1` to `4`_: its column of the order matrix:
   1. _As many as specified above_:
      1. **POINTER** — To a pattern.

[subpattern]: #patterns
[catalog]: #row-catalogs

## Patterns

A pattern is simply a collection of rows; [teNOR] attempts to find overlap between the patterns to minimise how much space they take; thus, all patterns are coalesced into sort of a "pool of rows".

There is no definitive end to the row pool—simply, only as many rows are emitted as are necessary.

Further, rows are _not_ emitted directly: instead, patterns contain indices that are used to index a "catalog" of rows.
(This enables separate copies of the same row to be stored more efficiently, but ends up imposing a limit of 256 unique cells across a track.)

### Row catalogs

Rows are composed of three bytes, stored in three 256-byte-aligned arrays.
(Yes, there is some amount of wasted padding between them. 😞 :sad_panda:)

There are two catalogs, one for rows belonging to the "main" patterns, and one for rows belonging to subpatterns; the contents of their arrays is slightly different between each:

#### Pattern rows

1. **BYTE** — The [effect]'s parameter.
1. **BYTE** — Split as follows:
   - **UPPER NIBBLE** — The instrument ID, or 0 for "none".
   - **LOWER NIBBLE** — The [effect] ID.
1. **BYTE** — The note's ID, or 90 for "no note".

#### Subpattern rows

1. **BYTE** — The [effect]'s parameter.
1. **BYTE** — Split as follows:
   - **UPPER NIBBLE** — Bits 0–3 of the next row's ID.
   - **LOWER NIBBLE** — The [effect] ID.
1. **BYTE** — Split as follows:
   - **BIT 7** — Bit 4 of the next row's ID.
   - **BITS 0–6** — The offset from the base note, plus 36; or 90 for "no offset".

In order to allow looping back on the same row as an effect, subpatterns rows all have a built-in jump target.
Since there are 32 possible rows to jump to, 5 bits are needed—the unused 7<sup>th</sup> bit of the note byte is used to store that 5<sup>th</sup> bit.

[effect]: #effects

#### Effects

Effect IDs are unchanged from hUGETracker.
The effect parameter is, however, sometimes different.

| Effect         | ID (hex) | Stored parameter                                                  |
| -------------- | -------- | ----------------------------------------------------------------- |
| Arpeggio       | 0        | Unchanged.                                                        |
| Porta up       | 1        | Unchanged.                                                        |
| Porta down     | 2        | Unchanged.                                                        |
| Tone porta     | 3        | Unchanged.                                                        |
| Vibrato        | 4        | Unchanged.                                                        |
| Set master vol | 5        | Unchanged.                                                        |
| Call routine   | 6        | Unchanged.                                                        |
| Note delay     | 7        | Unchanged.                                                        |
| Set panning    | 8        | Unchanged.                                                        |
| Change timbre  | 9        | Unchanged.                                                        |
| Vol slide      | A        | Unchanged.                                                        |
| Pos jump       | B        | The pattern ID is stored in [`wOrderIdx`](./internals.md) format. |
| Set vol        | C        | Nibbles are swapped from hUGETracker.                             |
| Pattern break  | D        | The row ID is stored in [`wForceRow`](./internals.md)'s format.   |
| Note cut       | E        | Unchanged.                                                        |
| Set tempo      | F        | Unchanged.                                                        |

## Instruments

Instruments are grouped in "banks" by their type.
Each bank is an array of up to 15 instruments, with no padding in-between.

### Duty

1. **BYTE** — Frequency sweep, in [NR10] format.
1. **BYTE** — Duty & length, in [NR11]/NR12 format.
1. **BYTE** — Volume & envelope, in [NR12]/NR22 format.
1. **POINTER** — Pointer to the [subpattern], or 0 if not enabled.
1. **BYTE** — Control bits:
   - **BIT 7** — Always set.
   - **BIT 6** — Whether the "length" is enabled.

### Wave

1. **BYTE** — Length, in [NR31] format.
1. **BYTE** — Volume, in [NR32] format.
1. **POINTER** — Pointer to the [subpattern], or 0 if not enabled.
1. **BYTE** — Control bits:
   - **BIT 7** — Always set.
   - **BIT 6** — Whether the "length" is enabled.
1. **BYTE** — ID of the [wave](#waves) to load.

### Noise

1. **BYTE** — Volume & envelope, in [NR42] format.
1. **POINTER** — Pointer to the [subpattern], or 0 if not enabled.
1. **BYTE** — Control bits:
   - **BIT 7** — 0 if the LFSR should be in "long" (15-bit) mode, 1 if the LFSR should be in "short" (7-bit) mode.
   - **BIT 6** — Whether the "length" is enabled.
   - **BITS 0–5** — Length, in [NR41] format.

## Waves

Each wave is an array of 16 bytes, stored directly in [wave RAM] format.

There are up to 16 waves; wave IDs start at 0.

## Routine

See [the dedicated chapter](./routines.md).

[teNOR]: ./teNOR.md
[NR10]: https://gbdev.io/pandocs/Audio_Registers.html#ff10--nr10-channel-1-sweep
[NR11]: https://gbdev.io/pandocs/Audio_Registers.html#ff11--nr11-channel-1-length-timer--duty-cycle
[NR12]: https://gbdev.io/pandocs/Audio_Registers.html#ff12--nr12-channel-1-volume--envelope
[NR31]: https://gbdev.io/pandocs/Audio_Registers.html#ff1b--nr31-channel-3-length-timer-write-only
[NR32]: https://gbdev.io/pandocs/Audio_Registers.html#ff1c--nr32-channel-3-output-level
[NR41]: https://gbdev.io/pandocs/Audio_Registers.html#ff20--nr41-channel-4-length-timer-write-only
[NR42]: https://gbdev.io/pandocs/Audio_Registers.html#ff21--nr42-channel-4-volume--envelope
[wave RAM]: https://gbdev.io/pandocs/Audio_Registers.html#ff30ff3f--wave-pattern-ram
