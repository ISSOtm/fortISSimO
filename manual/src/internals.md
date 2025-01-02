# Driver internals

This does not aim to document every single inner working of fortISSimO—there are too many of those, and too few people would be interested—it only contains descriptions of what each _variable_ does, since those are at least likely to be useful to people writing [routines](./routines.md).

Though I'm not opposed to the scope being expanded, if someone is interested.

> Last updated as of commit [`94a309b`](https://github.com/ISSOtm/fortISSimO/tree/94a309bb8d10e7947d9e2687a769cba9b89d2393).
> This is susceptible to having changed since then; see [the changes since then](https://github.com/ISSOtm/fortISSimO/compare/94a309bb8d10e7947d9e2687a769cba9b89d2393...master), particularly the file `src/fortISSimO.asm`.

**Be careful** of accessing these variables yourself if fortISSimO runs in an interrupt handler!
It may be possible for fortISSimO to execute between your code reading different bytes, and thus end up with an inconsistent state!

Note that the variable names follow [this naming guide](https://gbdev.io/guides/asmstyle.html#naming), except for variables exported by default (those with the `hUGE_` prefix).

## Song "cache"

All of the following variables are merely copied from the song header when `hUGE_SelectSong` is run, no processing applied!
(Their names should be self-explanatory as to what they contain.)
They can be modified at any time **between** two executions of `hUGE_TickSong`, and will take effect the next time they are read.

| Variable          | Accessed when?                                                                                                                      |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------- |
| `wTicksPerRow`    | On tick 0 of a new row.                                                                                                             |
| `wLastPatternIdx` | When "naturally" switching to the next pattern.                                                                                     |
| `wDutyInstrs`     | On tick 0 of a new row, if such an instrument must be loaded.                                                                       |
| `wWaveInstrs`     | On tick 0 of a new row, if such an instrument must be loaded.                                                                       |
| `wNoiseInstrs`    | On tick 0 of a new row, if such an instrument must be loaded.                                                                       |
| `wRoutine`        | Every time a "call routine" effect is executed.                                                                                     |
| `wWaves`          | When an instrument with a new wave (see `hUGE_LoadedWaveID`) is loaded, and every time a "change timbre" effect is executed on CH3. |

## Global variables

- `hUGE_LoadedWaveID`: ID of the wave the driver currently thinks is loaded in RAM, or `hUGE_NO_WAVE` for "none".
- `wArpState`: Which offset arpeggios should apply on this tick (1 = none, 2 = lower nibble, 3 = upper nibble); decremented before every tick.
- `wRowTimer`: Decremented before every tick, and if it reaches 0, a new row is switched to.
- `wOrderIdx`: Offset _in bytes_ within the order "columns"; since every entry (a pointer) is 2 bytes, this is always a multiple of 2.
- `wPatternIdx`: Which row in the current patterns is active, with bits 7 and 6 set. Incremented at the beginning of a tick where a new row is played.
- `wForceRow`: When switching to a new row, if this is set, this will be written to `wPatternIdx` instead of it being incremented.

## Channels

The channel variables are grouped under four structures, named `wCH1`, `wCH2`, `wCH3`, and `wCH4`; each member variable is a local label.

### Common

The following is common to all channels:

- `.order`: Pointer to this channel's order "column". Kind of part of the [song "cache"](#song-cache), but per-channel. Read every time a new row is switched to.
- `.fxParams`, `.instrAndFx`, `.note`: These cache the active row. (This avoids having to re-calculate the pointer to it and re-read it every time.)
- `.subPattern`: Pointer to the active instrument's subpattern, or 0 if disabled.
- `.subPatternRow`: Index of the active row in the subpattern (0–31).
- `.lengthBit`: This is OR'd into all bytes written to NRx4.

### Not CH4

The following is present in `wCH1`, `wCH2`, and `wCH3`, but not `wCH4`:

- `.period`: The current base "period", in the format that will get written to NRx3/NRx4. May not reflect what was last written to those registers, e.g. vibrato writes to those but not to this.
- To save some space, the following variables **overlap**, since they can't be used concurrently:
  - Used while a "tone porta" effect is active on this channel:
    - `.portaTarget`: The "period" that is being slid towards. This is redundant with `.note`, but serves as a cache.
  - Used while a "vibrato" effect is active on this channel:
    - `.vibratoOffset`: How much must be added to `.period` when writing to NRx3/NRx4.
    - `.vibratoState`: The upper nibble counts down before each tick, and the direction is flipped when it underflows. Bit 0 specifies the direction: clear when the period is increasing, and set when it's decreasing.
    - `.vibratoPrevArg`: If the previous row contained a vibrato, then this contains its argument; if not, then this has its lower nibble set to 0.

### CH4

The following is present on `wCH4` and only it.

- `.lfsrWidth`: The "LFSR width" bit, in [NR43] format.
- `.polynom`: The current "polynom", in [NR43] format (all bits but bit 3).

[NR43]: https://gbdev.io/pandocs/Audio_Registers.html#ff22--nr43-channel-4-frequency--randomness
