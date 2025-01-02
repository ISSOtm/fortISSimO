# Sound effects

fortISSimO does not include a built-in sound effect engine.
However, it has functionality to cooperate with any sound effect engine you want ([here is one](https://daid.github.io/gbsfx-studio/)): if a channel is "**muted**", then fortISSimO will _never_ access any of that channel's registers; this leaves it available for any other code, such as a sound effect engine!

A channel is considered "muted" if its corresponding bit is set in `hUGE_MutedChannels`; bit 0 controls CH1, bit 1 controls CH2, bit 2 controls CH3, and bit 3 controls CH4[^unused_bits].
(The constants `hUGE_CHx_MASK` are available (with <var>x</var> between 1 and 4) for your convenience.)

While a channel is "muted", all of its effects are processed, but any writes to hardware registers are discarded.
This means that "global" effects, such as `5xx`, `8xx`, `Fxx`, etc. are still applied properly.

When a channel is un-"muted", fortISSimO waits until a new "full" note (with instrument) is played on it to resume; this strategy avoids playing any corrupted sounds by accident, but can cause a channel to remain muted for a long time depending on the song's structure.

[^unused_bits]: The upper four bits of `hUGE_MutedChannels` are currently unused by fortISSimO; they may be repurposed in a future version, so for future-proofing/forward-compatibility, it is advisable not to touch them if possible.

## Wave RAM

The wave channel needs one extra precaution: if wave RAM is written to while CH3 is "muted", fortISSimO **must** be informed by setting `hUGE_LoadedWaveID` to the constant `hUGE_NO_WAVE`.
This will force it to reload wave RAM the next time a note is played on CH3.

## Stereo

Not only is a "set panning" (`8xx`) effect processed even on a muted channel, as explained above, its argument is _also_ written in full to [`NR51`].
This can interact poorly with sound effects, since it can alter the panning of a channel not meant for the sound driver at that time.

To remedy this, fortISSimO supports [the `FORTISSIMO_PANNING` tunable].
It should designate an address that panning info will be written to (in the usual `NR51` format, since `rNR51` is its default value).
That address _must_ be in HRAM, though.

You can set this to the address of a variable in HRAM, and implement "mixed" panning yourself.
For example, running the following code right after `hUGE_TickSound`:

```rgbasm
ldh a, [hUGE_MutedChannels] ; Assuming that all muted channels are used for SFX.
; Duplicate the lower nibble into the upper nibble.
ld c, a :: swap a :: or c
ld c, a

ldh a, [FORTISSIMO_PANNING]
or c ; Force all SFX channels to be centered.
ldh [rNR51], a
```

...or if you want stereo SFX:

```rgbasm
ldh a, [hUGE_MutedChannels] ; Assuming that all muted channels are used for SFX.
; Duplicate the lower nibble into the upper nibble.
ld c, a :: swap a :: or c
ld c, a

ldh a, [FORTISSIMO_PANNING]
ld b, a

ldh a, [hSfxPanning]
; "Bit mux": for each bit, if it's set in `c`, use the bit from `a`, otherwise from `b`.
xor b :: and c :: xor b ; Basically per-bit `c ? a : b`.
ldh [rNR51], a
```

[`NR51`]: https://gbdev.io/pandocs/Audio_Registers.html#ff25--nr51-sound-panning
[the `FORTISSIMO_PANNING` tunable]: ./integration.md#tuning-fortissimo
