# Routines

Routines are an advanced feature of fortISSimO, which allow you to execute custom code at arbitrary points in a track's playback.

fortISSimO diverges from hUGEDriver here in major ways: hUGEDriver supports 16 routines, fortISSimO supports only a single one; teNOR completely ignores the routines defined in the `.uge` file; and the interfaces provided to routines are very different.

## Setup

teNOR places each track's routine pointer at the very end of the generated `.asm` file. So, to define a routine, it's sufficient to `INCLUDE` the file and write the routine's code right after:

```rgbasm
INCLUDE "exports/boss_music.asm"
	; `600` disables the boss' invunlnerability, `601` enables it.
	ld a, b
	ld [wBossInvuln], a
	ret
```

## Interface

Routines are meant to be written in assembly code; no C wrapper is provided.
The routine is passed the entire effect's argument in the `b` register; no part of the argument is special, since there is only a single routine per song.

It is possible to use the argument to dispatch between several sub-routines, and even to mimic hUGEDriver's behaviour; for example:

```rgbasm
INCLUDE "exports/final_boss_music.asm"
	ld a, b
	and $F0
	jr z, .changeInvuln

	; `610` charges the boss' attack.
	; (This can run a lot more than you expect—
	;  please see the caveat in the next section.)
	ld hl, wBossChargeCounter
	inc [hl]
	ret

.changeInvuln
	; `600` disables the boss' invulnerability, `601` enables it.
	ld a, b
	ld [wBossInvuln], a
	ret
```

Additionally, `hl` points at the routine itself, `de` points at the channel's note byte ([`wCHx.note`](./internals.md)), and `c` contains the channel's mask ([`hUGE_CHx_MASK`](./sfx.md)).
The flags are not significant.

## When is the routine called?

Each active `6xx` effect causes the routine to be called once every tick, **not just the first one**!
Effects can come from the "main grid", where they are active for as many ticks as the tempo specifies, and/or from subpatterns, where they are only active for a single tick.

fortISSimO, unlike hUGEDriver, does not expose a tick counter by default.
You will have to [poke at the driver's internals](./internals.md) to obtain one.
