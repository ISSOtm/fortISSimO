# Integrating fortISSimO into a RGBDS project

Using fortISSimO, like many libraries, has three parts to it.

## Global init

fortISSimO has a few variables that must be initialised before some routines are called.
Forgetting to do so should result in uninitialised RAM being read (which your emulator is probably configured to warn you of).
The ideal time to initialise those is right after booting ([example](https://github.com/ISSOtm/fortISSimO-demo/blob/5463719e48580cc835d7459d607ee30056f51de8/src/main.asm#L17)).

- `hUGE_MutedChannels` must have been written to (usually to 0, but see [the chapter about sound effects](./sfx.md)) before `hUGE_TickSound` is ever called.

## Selecting a track

Here comes `hUGE_SelectSong`!
This function simply needs to be called with [the song's label](./teNOR.md#song-descriptor) in `de` ([example](https://github.com/ISSOtm/fortISSimO-demo/blob/5463719e48580cc835d7459d607ee30056f51de8/src/main.asm#L97-L98)).

This function's relationship with the APU is as follows:
- This function does not touch [`NR52`], so you must turn the APU on yourself (typically as part of the global init above, see [this example](ttps://github.com/ISSOtm/fortISSimO-demo/blob/5463719e48580cc835d7459d607ee30056f51de8/src/main.asm#L21)).
- This function does not touch [`NR51`] or [`NR50`] either; if your songs make use of panning, they should include `8xx` and/or `5xx` effects on their first row to reset those registers.

  Keep in mind that `8xx` and `5xx` are global, and thus affect sound effects as well!
- This function mutes every channel that is "owned" by the driver; if you do not want this (for example, to join two tracks seamlessly), set `hUGE_MutedChannels` to e.g. $0F before calling `hUGE_SelectSong`.

Additionally, `hUGE_TickSound` must not run in the middle of this function!
This can happen if it is called from an interrupt handler, notably.
The recommended fix is to "guard" calls to `hUGE_TickSound`, like this:

```rgbasm
	xor a
	ldh [hMusicReady], a
	ld de, BossFightMusic
	call hUGE_SelectSong
	ld a, 1
	ldh [hMusicReady], a
```

```rgbasm
	; In the interrupt handler:

	ldh a, [hIsMusicReady]
	and a
	call nz, hUGE_TickSound
```

Another possibility is to disable interrupt handlers (usually with `di` and `ei`) while `hUGE_SelectSong` is running; this can have side effects that affect your game, and is therefore not recommended.

## Playback

`hUGE_TickSound` is the function whose use requires the most attention.
Calling this function steps playback forward by 1 tick... which is the most fundamental unit of time in hUGETracker!

A given track expects this function to be called on a specific schedule, otherwise it will sound wrong.
Imagine playing a MP3 file at 1.5× speed, for example—that's not quite it, but close.

The schedule is simple:
- If "Enable timer-based tempo" was not selected in hUGETracker, then `hUGE_TickSound` must be called once per frame.
  This is most often done from an interrupt handler (preferably STAT to save VBlank time, but VBlank is fine too), but can also be done in the main loop.
- If "Enable timer-based tempo" was selected in hUGETracker, then `hUGE_TickSound` must be called at a fixed rate.
  This rate can be obtained by setting [`TAC`] to 4 (4096 Hz) and [`TMA`] to the value in the "Tempo (timer divider)" field, or any equivalent method.

Timer-based tempo can have annoying side effects to the rest of the game's programming, so VBlank-based tempo is recommended.

[`NR52`]: https://gbdev.io/pandocs/Audio_Registers.html#ff26--nr52-sound-onoff
[`NR51`]: https://gbdev.io/pandocs/Audio_Registers.html#ff25--nr51-sound-panning
[`NR50`]: https://gbdev.io/pandocs/Audio_Registers.html#ff24--nr50-master-volume--vin-panning
[`TAC`]: https://gbdev.io/pandocs/Timer_and_Divider_Registers.html#ff07--tac-timer-control
[`TMA`]: https://gbdev.io/pandocs/Timer_and_Divider_Registers.html#ff06--tma-timer-modulo
