# fortISSimO

A speed- and size-focused Game Boy music driver, intended for use with [hUGETracker](https://nickfa.ro/index.php/HUGETracker).

**Do you want to use fortISSimO? Then please read [the manual](https://eldred.fr/fortISSimO)!**

For an **example of usage**, as well as something to **quickly & easily make a ROM out of your song**, see [fortISSimO-demo](https://github.com/ISSOtm/fortISSimO-demo).

A table [comparing pros and cons of many GB music drivers](https://github.com/ISSOtm/fortISSimO/wiki/Drivers-comparison) is also available on the wiki.

This requires RGBDS 0.9.0 or later.

## Debugging features

If you're hacking on fortISSimO, you should use an emulator that supported [debugfiles], such as [Emulicious].
This will enable a lot of runtime checks.

Additionally, since fO stores music data in a fairly oblique way, you can define the variable `FORTISSIMO_LOG` when building (e.g. `rgbasm -DFORTISSIMO_LOG fortISSimO.asm`) to have the debugfile print every row that gets read.
(The format is arguably a little weird: subpatterns don't print the channel number but the channel “mask” instead; the note ID/offset is simply printed in decimal; and the instrument ID and FX ID are kind of just mashed together.)

## License

[![CC0 licensed (public domain)](https://licensebuttons.net/p/zero/1.0/80x15.png)](http://creativecommons.org/publicdomain/zero/1.0/)
To follow the license of hUGETracker and hUGEDriver, fortISSimO is dedicated to the public domain.

<p xmlns:dct="http://purl.org/dc/terms/" xmlns:vcard="http://www.w3.org/2001/vcard-rdf/3.0#">
  To the extent possible under law, all copyright and related or neighboring rights to
  <span property="dct:title">fortISSimO</span> have been waived.
  This work is published from <span property="vcard:Country" datatype="dct:ISO3166" content="FR" about="https://eldred.fr">France</span>.
</p>

[debugfiles]: https://github.com/aaaaaa123456789/gb-debugfiles/blob/master/debugfile.md
[Enulicious]: https://emulicious.net
