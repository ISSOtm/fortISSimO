# fortISSimO

A reimplementation from scratch of [SuperDisk's GB sound driver](https://github.com/SuperDisk/hUGEDriver/).

For documentation and example of usage, see [fortISSimO-demo](https://github.com/ISSOtm/fortISSimO).

## Quick start

- Required: RGBDS 0.3.8 or later
- Place this in any folder of your project
- If that folder is not the root of the project, add a `-i` flag for it
- Include `main.asm`
- Before any calls to `hUGE_TickSound` are made, either set `whUGE_Enabled` to 0, or call `hUGE_StartSong`
