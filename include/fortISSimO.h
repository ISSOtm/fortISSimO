#ifndef FORTISSIMO_H
#define FORTISSIMO_H

#include <gbdk/platform.h>

// fortISSimO routines take no parameters, and return nothing.
typedef void (*hUGERoutine_t)();

// fortISSimO songs are opaque blobs: you should only manipulate pointers to them.
// (You *can* manipulate them if you try hard enough, but the format will change between releases.)
struct hUGESong;

/**
 * Set up the driver to begin playing a song.
 */
void hUGE_StartSong(struct hUGESong const * song) __sdcccall(1); // Use the register calling convention.

/**
 * Calling this advances song playback by one tick.
 * This should be called as specified in the tracker: either once per frame (either from the main
 * loop, or from the VBlank or STAT interrupts), or on a timer (usually using the timer interrupt).
 *
 * The former is advised.
 *
 * IMPORTANT: this must NOT be called while `hUGE_init` is executing!
 *            If you are calling this from an interrupt handler, you should use a variable to
 *            avoid this race condition:
 * ```c
 * bool driverBusy = false;
 *
 * void playNewSong(uint16_t songID) {
 *     driverBusy = true;
 *     hUGE_StartSong();
 *     driverBusy = false;
 * }
 *
 * void vblankIntHandler() { // For example
 *     // ...
 *
 *     if (!driverBusy) {
 *         hUGE_TickSound();
 *     }
 *
 *     // ...
 * }
 * ```
 */
void hUGE_TickSound();

/**
 * As soon as a channel's bit gets set in this variable, the driver will stop updating any of its registers.
 * This is useful, for example, for playing sound effects: temporarily "mute" one of the song's channels,
 * play the SFX on it, and un-mute it to "release" it back to the driver.
 *
 * Note that the driver will not touch the channel until a new note plays on it.
 *
 * IMPORTANT: musing CH3 (the wave channel) requires special attention: if wave RAM is modified, you
 *            must call `hUGE_ResetWave()` before unmuting the channel.
 */
// TODO: allow using the ASM mask constants somehow.
extern unsigned char hUGE_MutedChannels __sfr; // TODO: I think this is how you tag HRAM?

static inline void hUGE_ResetWave() {
	extern unsigned char hUGE_LoadedWaveID;

	hUGE_current_wave = 100; // TODO: use the constant from the ASM file instead
}

#endif
