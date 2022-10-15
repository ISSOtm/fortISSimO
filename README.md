# fortISSimO

[![CC0 licensed (public domain)](https://licensebuttons.net/p/zero/1.0/80x15.png)](http://creativecommons.org/publicdomain/zero/1.0/)

A reimplementation from scratch of [SuperDisk's GB sound driver](https://github.com/SuperDisk/hUGEDriver/).

For documentation and example of usage, see [fortISSimO-demo](https://github.com/ISSOtm/fortISSimO-demo).

## Quick start

- Required: RGBDS 0.5.0 or later
- Place this in any folder of your project
- If that folder is not the root of the project, add a `-i` flag for it
- Include `main.asm`
- Before any calls to `hUGE_TickSound` are made, either set `whUGE_Enabled` to 0, or call `hUGE_StartSong`

## License

<p xmlns:dct="http://purl.org/dc/terms/" xmlns:vcard="http://www.w3.org/2001/vcard-rdf/3.0#">
  To the extent possible under law, all copyright and related or neighboring rights to
  <span property="dct:title">fortISSimO</span> have been waived.
  This work is published from <span property="vcard:Country" datatype="dct:ISO3166" content="FR" about="https://eldred.fr">France</span>.
</p>
