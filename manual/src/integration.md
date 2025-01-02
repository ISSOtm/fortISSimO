# Integration

Integrating fortISSimO into your project depends on what toolchain you are using; please go to the appropriate page for detailed instructions.

The following, however, is independent of the toolchain.

## Debugfile support

fortISSimO supports [debugfiles], which enable supporting emulators (such as [Emulicious]) to perform many run-time sanity checks for free.
This can help catch bugs in fortISSimO, songs, or custom routines.

Define a `PRINT_DEBUGFILE` symbol (e.g. by passing [`-DPRINT_DEBUGFILE`](https://rgbds.gbdev.io/docs/rgbasm.1#D) to `rgbasm`) to have the debugfile printed to standard output.

So, for example:

```console
$ rgbasm src/fortISSimO.asm -I src/include -DPRINT_DEBUGFILE >obj/fortISSimO.dbg
```

[debugfiles]: https://github.com/aaaaaa123456789/gb-debugfiles
[Emulicious]: https://emulicious.net

## Tuning fortISSimO

fortISSimO supports a bit of configuration without having to modify `fortISSimO.asm`, which would make upgrading more difficult.

The following symbols can/must be defined when assembling `fortISSimO.asm`:

| Name                  | Kind                                | Default     | Functionality                                                                                                                                                                                                                                                            |
| --------------------- | ----------------------------------- | ----------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `FORTISSIMO_ROM`      | [String constant]                   | `ROM0`      | Attributes for fortISSimO's ROM [section].<br/>Example: `ROMX, BANK[42]`.<br/>If empty, **no `SECTION` directive will be emitted**, which can be useful if doing `INCLUDE "fortISSimO.asm"`.                                                                             |
| `FORTISSIMO_RAM`      | [String constant]                   | `WRAM0`     | Attributes for fortISSimO's RAM [section].<br/>Example: `WRAMX, ALIGN[4]`.                                                                                                                                                                                               |
| `FORTISSIMO_CH3_KEEP` | Any                                 | Not defined | If any symbol by this name is defined, then fortISSimO will **not** remove CH3 from [NR51] temporarily while writing to wave RAM. This may make the process sound slightly "clicky", but allows `hUGE_TickSound` to be safely interrupted by code that writes to [NR51]. |
| `FORTISSIMO_PANNING`  | [String constant] or numeric symbol | `rNR51`     | Where fortISSimO's "set panning" effect (`4xx`) will write <var>xx</var> to. This can be useful for [sound effect integration].                                                                                                                                          |

[String constant]: https://rgbds.gbdev.io/docs/rgbasm.5#Strong_constants
[section]: https://rgbds.gbdev.io/docs/rgbasm.5/#SECTIONS
[NR51]: https://gbdev.io/pandocs/Audio_Registers.html#ff25--nr51-sound-panning
[sound effect integration]: ./sfx.md
