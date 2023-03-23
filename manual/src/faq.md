# FAQ

<style>
	h2::before { content: "Q: "; font-weight: bolder; }
	h2 + *::before { content: "A: "; font-weight: bold; }
	dt { font-weight: bold; }
</style>

## Why is this called "fortISSimO"?

It started with a joke by [SuperDisk](https://nickfa.ro), when we talked about my intention to reimplement [hUGEDriver](https://github.com/SuperDisk/hUGEDriver) "but more optimised".
He suggested something that contained my nickname (ISSOtm), and we eventually landed on fortISSimO; the weird capitalisation is meant to make the funny more obvious, and is par for the course for the hUGETracker ecosystem 😛

## Are there any conditions to using this in my game?

Basically, none!
fortISSimO was placed in the public domain, since both hUGETracker and hUGEDriver did the same.
Mentioning us in your credits, special thanks, or similar would however be very appreciated.

You are free to incorporate fortISSimO into whatever, and modify it as well!
As well, contributing back (typically with [a pull request](https://docs.github.com/en/pull-requests/collaborating-with-pull-requests/proposing-changes-to-your-work-with-pull-requests/creating-a-pull-request)) would be very appreciated.

## What do I stand to gain from using <abbr title="fortISSimO">fO</abbr> over <abbr title="hUGEDriver">hD</abbr>?

The first upside over [choosing another driver outright](https://github.com/ISSOtm/fortISSimO/wiki/Drivers-comparison) is that you remain in the [hUGETracker](https://github.com/SuperDisk/hUGETracker) ecosystem; both <abbr title="hUGEDriver">hD</abbr> and <abbr title="fortISSimO">fO</abbr> have the same high-level interface, so switching between the two is largely painless, whereas switching to another driver would likely mean re-composing your songs on top of more involved integration changes.

All other things equal, <abbr title="hUGEDriver">hD</abbr> vs <abbr title="fortISSimO">fO</abbr> is a tradeoff:

### New releases

hUGEDriver is the canonical driver for hUGETracker, which implies that it will always be up to date with hUGETracker updates.
Comparatively, fortISSimO may be lagging behind, as it's maintained by a third party (me).

I'm however striving to keep the pace; feel free to [open an issue](https://github.com/ISSOtm/fortISSImO/issues) if fortISSimO appears to be significantly out of date.

### Design priorities

hUGEDriver is programmed rather straightforwardly, which makes *some* bugs less likely to appear.
fortISSimO is programmed to be **fast and small** first, and easy to maintain second.[^maintainable]

Optimisations have an impact on three different aspects:

<dl>
<dt>Speed</dt>
<dd>

The music driver typically runs on every frame, so every CPU cycle it uses has a significant impact on how much room the game has to run logic and updates.

</dd>
<dt>Driver size</dt>
<dd>

It's highly rare for all music data to fit in a single ROM bank, so the music driver is almost placed in ROM bank 0, in which space tends to be quite precious.

</dd>
<dt>Track size</dt>
<dd>

Since music tends to be placed in ROMX, most games actually don't care too much about the actual music's size (though smaller doesn't hurt).
However, people making ["ROM-only"](https://gbdev.io/pandocs/nombc.html) games have a small, hard cap of 32 KiB on total size; smaller music data helps this use case.[^compression]

</dd>
</dl>

[^maintainable]: fortISSimO is still written to be maintainable: there are several potential optimisations that were rejected because they would make a real mess of the code. But I've been pushing the optimisation/maintainability slider further than most people would, is what I meant to say.

[^compression]: Those people have had a tendency to actually compress the music data; it is still worth making the uncompressed data smaller, since that almost always leads to smaller compressed data as well, and also faster decompression (if only because less data has to be read and written).

### Human-readable music data

SuperDisk stated that hUGEDriver embeds the input data mostly as-is in the ROM as a design feature—the binary data is more human-readable and easier to track the origin of.

fortISSimO instead believes that hardly anyone looks at the binary data, so it's not worth optimising for that use case.

It also seems that composers tend to be sloppy and produce tracks with unused or redundant instruments, patterns, etc; __this is perfectly legitimate__, since they *iterate* on the file itself, and thus it makes more sense to prioritise convenience and wiggle room over data optimisation.

For all these reasons, fortISSimO opts to pre-process the music data when building the ROM, whenever this enables size reduction or faster processing.
