# Differences with hUGEDriver

fortISSimO aims to be as close to hUGEDriver as possible, but sports a few differences.
_As of hUGETracker 1.0.11, anyway—hUGEDriver might choose to implement some of all of these changes in a future release!_

If you notice any difference not listed in this page, it's likely a bug!
Please [open an issue](https://github.com/ISSOtm/fortISSimO/issues/new), or contact me.

## Vibrato

Vibrato works quite differently under fortISSimO.

- Vibrato is **not** supported at all in subpatterns!
- fortISSimO produces a triangle vibrato (hUGEDriver's is square).[^vib_shape]
- As a consequence, the vibrato's parameter is interpreted differently:
  > For a `4xy` effect, `x` indicates the vibrato's rate, and `y` its slope: for `x` ticks, the frequency will be increased by `y` units each tick; then for `x` ticks, the frequency will be decreased by `y` units each tick.
- A vibrato is restarted at the beginning of its row, _except_ if the previous row had a vibrato with exactly the same parameter.

[^vib_shape]: For those who prefer square vibratos: sorry, but the vibrato shape is baked into the driver itself—it avoids using a LUT for size's sake—so you can't change it without somewhat involved modifications to `fortISSimO.asm`.

## Tone portamento

On a row that contains the tone portamento effect (`3xy`) _and_ an instrument ID, hUGEDriver reloads the instrument's parameters; fortISSimO instead ignores the instrument.

## Subpatterns

The "set speed" effect (`Fxx`) is not supported in subpatterns.

Additionally, fortISSimO fixes a bug in hUGEDriver where any jumps to row #31 (`J32` in the tracker) would be ignored.

## "Absolute" subpatterns

fortISSimO's subpatterns allow temporarily overriding the current note!
This is done with effect `7xx` (normally "note delay", unavailable in subpatterns); the parameter indicates which note shall be used on that row (the lower the argument, the lower the pitch).

Note that using this effect overrides every other modification to the pitch for the tick where it's active.
