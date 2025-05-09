; If you want to use fortISSimO inside of hUGETracker itself,
; please read the dedicated section in the manual:
; https://eldred.fr/fortISSimO/hugetracker.html
;
; def HUGETRACKER equs "???"

IF DEF(HUGETRACKER)
	WARN "\n\tPlease report this issue to fortISSimO, *NOT* hUGETracker!\n\t(Even if it seems unrelated.)\n\t>>> https://github.com/ISSOtm/fortISSimO/issues <<<\n"
	IF !STRCMP("{HUGETRACKER}", "1.0b10")
	ELIF !STRCMP(STRSUB("{HUGETRACKER}", 1, 3), "1.0")
	ELSE
		FAIL "Unsupported hUGETracker version \"{HUGETRACKER}\"!"
	ENDC
ELIF DEF(PREVIEW_MODE)
	; PREVIEW_MODE is defined when assembling for hUGETracker.
	; hUGETracker contains a Game Boy emulator (the "G" in "UGE"), and it relies on some cooperation
	; from the driver to signal key updates.
	; This goes both ways, though: don't try to run PREVIEW_MODE code outside of hUGETracker!
	FAIL "fortISSimO is not properly configured for use in hUGETracker!\n\tPlease follow the instructions at the top of hUGEDriver.asm."
ENDC


; Some terminology notes:
; - An "order" is a table of pointers to "patterns".
; - A "pattern" is a series of up to 64 "rows".
; - A "table" is a subpattern
;
; A "TODO: loose" comment means that the `hli` it's attached to could be used as an optimisation point.


IF !DEF(HUGETRACKER)
	INCLUDE "hardware.inc" ; Bread & butter: check.
	INCLUDE "fortISSimO.inc" ; Get the note constants.
ELSE ; The above files are accessed differently when inside hUGETracker.
	INCLUDE "include/hardware.inc"
	INCLUDE "include/hUGE.inc"
	IF !DEF(FORTISSIMO_INC)
		FAIL "It seems that you forgot to overwrite hUGETracker's `hUGE.inc` with `fortISSimO.inc`!"
	ENDC
ENDC


; Some configuration, with defaults.
; See https://eldred.fr/fortISSimO/integration.html#tuning-fortissimo for details.
IF !DEF(FORTISSIMO_ROM)
	def FORTISSIMO_ROM equs "ROM0"
ENDC
IF !DEF(FORTISSIMO_RAM)
	def FORTISSIMO_RAM equs "WRAM0"
ENDC
IF !DEF(FORTISSIMO_PANNING)
	def FORTISSIMO_PANNING equ rNR51
ENDC


	rev_Check_hardware_inc 4.2


IF DEF(override)
	PURGE override
ENDC
MACRO override ; Ensures that fO's symbols don't conflict with pre-defined ones.
	REPT _NARG
		IF DEF(\1)
			PURGE \1
		ENDC
		shift
	ENDR
ENDM
override dbg_var, dbg_action, dbg_log, runtime_assert, unreachable ; Pre-defined ones, if any, may have different semantics.

IF DEF(PRINT_DEBUGFILE)
	PRINTLN "@debugfile 1.0.0"

	MACRO dbg_var ; <name>, <default value>
	    DEF DEFAULT_VALUE equs "0"
	    IF _NARG > 1
	        redef DEFAULT_VALUE equs "\2"
	    ENDC
	    PRINTLN "@var \1 {DEFAULT_VALUE}"
	    PURGE DEFAULT_VALUE
	ENDM

	MACRO dbg_action ; <action:str> [, <condition:dbg_expr>]
		DEF OFS_FROM_BASE equ @ - {.}
		DEF ACTION_COND equs ""
		IF _NARG > 1
			REDEF ACTION_COND equs "\2"
		ENDC
		PRINTLN "{.}+{d:OFS_FROM_BASE} x {ACTION_COND}: ", \1
		PURGE OFS_FROM_BASE, ACTION_COND
	ENDM

ELSE ; If not printing debugfiles to stdout, define the "core" macros as do-nothing.
	MACRO dbg_var
	ENDM
	MACRO dbg_action
	ENDM
ENDC

MACRO dbg_log ; <message:dbg_str> [, <condition:dbg_expr>]
	DEF MSG equs \1
	SHIFT
	dbg_action "message \"{MSG}\"", \#
	PURGE MSG
ENDM

MACRO runtime_assert ; <condition:dbg_expr> [, <message:dbg_str>]
	DEF MSG equs "assert failure"
	IF _NARG > 1
		REDEF MSG equs \2
	ENDC
	dbg_action "alert \"{MSG}\"", !(\1)
	PURGE MSG
ENDM

MACRO unreachable ; [<message:dbg_str>]
	DEF MSG equs "unreachable code reached!"
	IF _NARG > 0
		REDEF MSG equs \1
	ENDC
	dbg_action "alert \"In {.}: {MSG}\""
	PURGE MSG
ENDM


IF !DEF(FORTISSIMO_LOG)
	def FORTISSIMO_LOG equs ""
ELSE
	redef FORTISSIMO_LOG equs ",{FORTISSIMO_LOG},"
ENDC
MACRO fO_log ; <category:name>, <message:dbg_str> [, <condition:dbg_expr>]
	IF STRIN("{FORTISSIMO_LOG}", ",\1,")
		shift
		dbg_log \#
	ENDC
ENDM



; Note: SDCC's linker is crippled by the lack of alignment support.
; So we can't assume any song data nor RAM variables are aligned, as useful as that would be.
;
; SDCC calling conventions: https://sdcc.sourceforge.net/doc/sdccman.pdf#subsubsection.4.3.5.1

IF STRLEN("{FORTISSIMO_ROM}") != 0
	SECTION "Sound Driver", FORTISSIMO_ROM
ENDC

_hUGE_SelectSong:: ; C interface.
; @param de: Pointer to the "song descriptor" to load.
; @destroy af bc de hl
hUGE_SelectSong::
	; Kill channels that aren't muted.
	; Note that we re-enable the channels right after, to avoid pops when they come back online.
	ldh a, [hUGE_MutedChannels]
	ld hl, rNR12
	ld bc, AUDENV_UP ; B = 0
	rra
	jr c, .ch1NotOurs
	ld [hl], b
	ld [hl], c
.ch1NotOurs
	rra
	jr c, .ch2NotOurs
	ld l, LOW(rNR22)
	ld [hl], b
	ld [hl], c
.ch2NotOurs
	rra
	jr c, .ch3NotOurs
	ld l, LOW(rNR30)
	ld [hl], l ; This has bit 7 reset.
	ld [hl], h ; This has bit 7 set.
.ch3NotOurs
	rra
	jr c, .ch4NotOurs
	ld l, LOW(rNR42)
	ld [hl], b
	ld [hl], c
.ch4NotOurs
	ld hl, hUGE_LoadedWaveID
	ld a, hUGE_NO_WAVE
	ld [hli], a

	xor a ; Begin by not touching any channels until a note first plays on them.
	ldh [hUGE_AllowedChannels], a

	; Set arpeggio state to something.
	assert hUGE_LoadedWaveID + 1 == wArpState
	inc a ; ld a, 1
	ld [hli], a

	assert wArpState + 1 == wRowTimer
	; a = 1
	ld [hli], a ; The next tick will switch to a new row.

	assert wRowTimer + 1 == wTicksPerRow
	ld a, [de]
	ld [hli], a
	inc de

	assert wTicksPerRow + 1 == wLastPatternIdx
	ld a, [de]
	ld [hli], a
	add a, 2
	ld b, a ; Cache the size of each order table for later.
	inc de

	assert wLastPatternIdx + 1 == wDutyInstrs
	assert wDutyInstrs + 2 == wWaveInstrs
	assert wWaveInstrs + 2 == wNoiseInstrs
	assert wNoiseInstrs + 2 == wRoutine
	assert wRoutine + 2 == wWaves
	assert wWaves + 2 == wRowCatalogHigh
	assert wRowCatalogHigh + 1 == wSubpatRowCatalogHigh
	ld c, 2 + 2 + 2 + 2 + 2 + 1 + 1
.copyPointers
	ld a, [de]
	ld [hli], a
	inc de
	dec c
	jr nz, .copyPointers

	assert wSubpatRowCatalogHigh + 1 == wOrderIdx
IF DEF(PREVIEW_MODE)
	; The tracker writes the starting order, but `wForceRow` will cause it to increase.
	assert wOrderIdx == current_order
	ld a, [hl]
	sub 2
ELSE
	; Begin at order 0, but `wForceRow` will cause it to increase.
	ld a, -2
ENDC
	ld [hli], a
	assert wOrderIdx + 1 == wPatternIdx
	inc hl ; No need to init that, it will be set from `wForceRow`.
	assert wPatternIdx + 1 == wForceRow
	assert PATTERN_LENGTH == 1 << 6, "Pattern length must be a power of 2"
IF DEF(PREVIEW_MODE)
	ld a, [row]
	or -PATTERN_LENGTH
ELSE
	ld a, -PATTERN_LENGTH
ENDC
	ld [hli], a ; Begin by forcing row 0.

	; Time to init the channels!
	assert wForceRow + 1 == wCH1
	ld c, 4
.initChannel
	assert wCH1 == wCH1.order
	; Copy the order pointer.
	ld a, e
	ld [hli], a
	add a, b
	ld e, a
	ld a, d
	ld [hli], a
	adc a, 0
	ld d, a
	assert wCH1.order + 2 == wCH1.fxParams
	inc hl ; Skip FX params.
	assert wCH1.fxParams + 1 == wCH1.instrAndFX
	; The FX is checked on the first tick for whether it is a vibrato; set it to 0, which is not that.
	assert FX_VIBRATO != 0
	xor a
	ld [hli], a
	assert wCH1.instrAndFX + 1 == wCH1.note
	inc hl ; Skip note ID.
	assert wCH1.note + 1 == wCH1.subPattern

	; To ensure that nothing bad happens if a note isn't played on the first row, set the subpattern
	; pointer to NULL.
	; xor a ; a is already 0.
	ld [hli], a
	ld [hli], a
	assert wCH1.subPattern + 2 == wCH1.subPatternRow
	; Although strictly speaking, init'ing the subpattern row is unnecessary, it's still read before
	; the NULL check is performed; doing this silences any spurious "uninit'd RAM read" exceptions.
	ld [hli], a
	assert wCH1.subPatternRow + 1 == wCH1.lengthBit
	; Same as above.
	ld [hli], a
	; Then, we have the 4 channel-dependent bytes
	; (period + (porta target / vib counter) / LFSR width + polynom + padding); they don't need init.
	ld a, l
	add a, 5
	ld l, a
	adc a, h
	sub l
	ld h, a
	assert wCH1.lengthBit + 1 + 5 == wCH2

	dec c ; Are we done?
	jr nz, .initChannel
	ret


_hUGE_TickSound:: ; C interface.
hUGE_TickSound::
	; Disable all muted channels.
	ld hl, hUGE_MutedChannels
	ld a, [hli]
	assert hUGE_MutedChannels + 1 == hUGE_AllowedChannels
	cpl
	and [hl]
	ld [hl], a

	ld hl, wArpState
	dec [hl]
	jr nz, :+
	ld [hl], 3
:
	inc hl
	assert wArpState + 1 == wRowTimer

	; Check if we should switch to a new row, or just update "continuous" effects.
	dec [hl]
	jp nz, ContinueFx


	;; This is the first tick; switch to the next row, and reload all pointers.

	; Reload delay.
	ld a, [wTicksPerRow]
	ld [hli], a ; TODO: loose

	; Check if there is a row or pattern break, and act accordingly:
	; Pattern break + row break at the same time must switch to row R on pattern P!
	; But row break on last row must not change the pattern.

	; Switch to next row.
	ld hl, wForceRow
	ld a, [hld]
	assert wForceRow - 1 == wPatternIdx
	and a
	jr nz, .forceRow
	inc [hl]
	jr nz, .samePattern
	; Reload index.
	assert PATTERN_LENGTH == 1 << 6, "Pattern length must be a power of 2"
	ld a, -PATTERN_LENGTH ; pow2 is required to be able to mask off these two bits.
.forceRow
	ld [hld], a
IF DEF(PREVIEW_MODE)
	jr nz, .incRequired ; Everything that sets `wForceRow` expects the order to advance.
	inc hl
	; If looping is enabled, don't switch patterns.
	ld a, [loop_order]
	and a
	jr nz, .samePattern
	dec hl
.incRequired
ENDC
	; Switch to next patterns.
	assert wPatternIdx - 1 == wOrderIdx
	ld a, [wLastPatternIdx]
	sub [hl]
	jr z, .wrapOrders ; Reached end of orders, start again from the beginning.
	ld a, [hl]
	assert ORDER_WIDTH == 2
	inc a
	inc a
.wrapOrders
	ld [hli], a
	assert wOrderIdx + 1 == wPatternIdx
	IF DEF(PREVIEW_MODE)
		db $fc ; Signal the tracker to refresh the order index.
	ENDC
.samePattern
	; Compute the offset into the pattern.
	ld a, [hli]
	assert PATTERN_LENGTH == 1 << 6, "Pattern length must be a power of 2"
	and PATTERN_LENGTH - 1
	fO_log row_idx, "=== Order row \{[wOrderIdx] / 2 + 1\}, row \{a\} ==="
	ld b, a
	; Reset the "force row" byte.
	assert wPatternIdx + 1 == wForceRow
	xor a
	ld [hli], a


	;; Play new rows.

	; Note that all of these functions leave b untouched all the way until CH4's `ReadRow`!

	; If the previous FX was not a vibrato, set the "vibrato arg" to 0.
	; Note that all of these should run on the song's first tick, which initialises `.vibratoPrevArg`
	; if the first row has a vibrato, and avoids reading uninit'd RAM.
	; Note also that these are all run before `RunTick0Fx`, which can write to the overlapping `.portaTarget`,
	; and before `ReadRow`, which will overwrite `.instrAndFx`.
	ld a, [wCH1.instrAndFX]
	and $0F
	cp FX_VIBRATO
	jr z, .ch1WasNotVibrato
	xor a
	ld [wCH1.vibratoPrevArg], a
.ch1WasNotVibrato

	ld a, [wCH2.instrAndFX]
	and $0F
	cp FX_VIBRATO
	jr z, .ch2WasNotVibrato
	xor a
	ld [wCH2.vibratoPrevArg], a
.ch2WasNotVibrato

	ld a, [wCH3.instrAndFX]
	and $0F
	cp FX_VIBRATO
	jr z, .ch3WasNotVibrato
	xor a
	ld [wCH3.vibratoPrevArg], a
.ch3WasNotVibrato
	; CH4 does not support vibrato, so it's not checked.

	assert wForceRow + 1 == wCH1.order
	; ld hl, wCH1.order
	call ReadRow
	ld hl, wCH1.instrAndFX
	ld e, hUGE_CH1_MASK
	ld c, LOW(rNR10)
	call nz, PlayDutyNote

	ld hl, wCH2.order
	call ReadRow
	ld hl, wCH2.instrAndFX
	ld e, hUGE_CH2_MASK
	ld c, LOW(rNR21 - 1) ; NR20 doesn't exist.
	call nz, PlayDutyNote

	ld hl, wCH3.order
	call ReadRow
	call nz, PlayWaveNote

	ld hl, wCH4.order
	call ReadRow
	call nz, PlayNoiseNote


	;; Process tick 0 FX and subpatterns.

	ld de, wCH1.fxParams
	ld c, hUGE_CH1_MASK
	call RunTick0Fx
	ld hl, wCH1.lengthBit
	ld c, hUGE_CH1_MASK
	call TickSubpattern

	ld de, wCH2.fxParams
	ld c, hUGE_CH2_MASK
	call RunTick0Fx
	ld hl, wCH2.lengthBit
	ld c, hUGE_CH2_MASK
	call TickSubpattern

	ld de, wCH3.fxParams
	ld c, hUGE_CH3_MASK
	call RunTick0Fx
	ld hl, wCH3.lengthBit
	ld c, hUGE_CH3_MASK
	call TickSubpattern

	ld de, wCH4.fxParams
	ld c, hUGE_CH4_MASK
	call RunTick0Fx
	ld hl, wCH4.lengthBit
	ld c, hUGE_CH4_MASK
	assert @ == TickSubpattern ; fallthrough


; @param hl: Pointer to the channel's length bit.
; @param c:  The channel's mask (the CHx_MASK constant).
; @destroy a bc de hl (potentially)
TickSubpattern:
	ld a, [hld] ; Read the length bit.
	ld b, a
	assert wCH1.lengthBit - 1 == wCH1.subPatternRow
	runtime_assert [@hl] < 32, "Subpattern row index out of bounds! (\{[@hl]\})"
	ld a, [hld]
	ld e, a
	assert wCH1.subPatternRow - 2 == wCH1.subPattern ; 16-bit variable.
	; Add the row offset to the subpattern base pointer.
	ld a, [hld]
	ld d, a
	or [hl]
	ret z ; Return if subpattern pointer is NULL (no subpattern).

	ld a, [hld]
	assert wCH1.subPattern - 1 == wCH1.note
	push hl ; Save pointer to current note.
	add a, e
	ld l, a
	adc a, d
	sub l
	ld h, a
	; Read the row's ID, and compute the pointer to it.
	ld l, [hl]
	ld a, [wSubpatRowCatalogHigh]
	ld h, a
	fO_log subpat_row, "CHx_MASK=\{c\} reading subpattern row from $\{hl,04$\}: (\{[hl + 512],2\}, \{[hl + 256],2$\}_\{[hl],2$\})"
	; Read the row's FX parameter.
	ld a, [hl]
	ldh [hUGE_FxParam], a
	inc h
	runtime_assert [(([@hl] & $0F) * 2 + TickSubpattern.fxPointers)!] != KnownRet, "Bad command (\{[@hl],$\}) in subpattern!"
	ld a, [hl] ; Read the jump target and FX ID.
	inc h
	ld l, [hl] ; Read the note offset.
	IF DEF(HUGETRACKER)
		rlc l ; We can't store the offset pre-rotated because hT uses a single `dn` macro.
	ENDC
	ld h, a ; We'll need to persist this for a bit.

	; Update the index to point to the next row.
	and $F0 ; Retain the jump target only.
	; There is one extra bit (bit 4) in the note field, specifically its bit 0.
	srl l ; Move the extra bit into carry, and put the note in place.
	adc a, 0 ; Inject the extra bit into bit 0.
	swap a ; Put the bits in their right place.
	pop de ; This points to `wCHx.note`.
	assert wCH1.subPatternRow - wCH1.note == 3
	inc de
	inc de
	inc de
	IF DEF(HUGETRACKER) ; We don't normally do this, because it introduces an overflow bug.
		jr nz, .jump
		ld a, [de]
		inc a
		and 32 - 1 ; Subpatterns wrap around.
		db $FE ; Swallow up next byte.
	.jump
		dec a
	ENDC
	ld [de], a

	; Apply the note offset, if any.
	dec de ; Point back to the base note.
	dec de
	dec de
	ld a, l
	cp LAST_NOTE
	jr nc, .noNoteOffset
	; Check if the channel is muted; if so, don't write to NRxy.
	ldh a, [hUGE_AllowedChannels]
	and c
	jr z, .noNoteOffset
	; Compute the note's ID.
	ld a, [de]
	add a, l
	sub LAST_NOTE / 2 ; Go from "unsigned range" to "signed range".
	runtime_assert @a < {LAST_NOTE}, "Subpattern offset over/underflowed note ID! \{@a\}"
	bit 3, c
	assert hUGE_CH4_MASK == 1 << 3
	jr nz, .ch4
	; For the FX dispatch below, we need the FX ID (in `h`) and the channel mask (in `c`).
	ld l, c
	push hl
	push de
	ld hl, wCH1.period - wCH1.note
	add hl, de
	; Compute the note's period.
	add a, a
	add a, LOW(PeriodTable)
	ld e, a
	adc a, HIGH(PeriodTable)
	sub e
	ld d, a
	; Compute the pointer to NRx3, bit twiddling courtesy of @calc84maniac.
	ld a, c ; a = 1 (CH1), 2 (CH2), or 4 (CH3).
	xor $11  ; 10, 13, 15
	add a, c ; 11, 15, 19
	cp LOW(rNR23)
	adc a, c ; 13, 18, 1D
	ld c, a
	; Write the period, together with the length bit.
	ld a, [de]
	ld [hli], a
	ldh [c], a
	inc c
	inc de
	ld a, [de]
	ld [hl], a
	or b ; Add the length bit.
	ldh [c], a
	pop de ; Restore the pointer to the channel's note ID (for FX).
	pop bc ; Restore the FX ID and the channel mask.
.appliedOffset

	; Play the row's FX.
	ld a, b ; Read the FX/instr byte again.
	and $0F ; Keep the FX bits only.
	add a, a
	add a, LOW(.fxPointers)
	ld l, a
	adc a, HIGH(.fxPointers)
	sub l
	ld h, a
	; Retrieve the FX param.
	ldh a, [hUGE_FxParam]
	ld b, a
	; Deref the pointer, and jump to it.
	ld a, [hli]
	ld h, [hl]
	ld l, a
	jp hl

.ch4
	call GetNoisePolynom
	ld d, a
	ld [wCH4.polynom], a
	ld a, [wCH4.lfsrWidth]
	or d
	ldh [rNR43], a
	ld a, b
	ldh [rNR44], a
	ld de, wCH4.note ; Restore the pointer to the channel's note ID (for FX).
.noNoteOffset
	ld b, h ; Transfer the FX ID for the calling code.
	jr .appliedOffset

.fxPointers
	dw FxArpeggio
	dw FxPortaUp
	dw FxPortaDown
	dw KnownRet ; No tone porta
	dw KnownRet ; No vibrato
	dw FxSetMasterVolume
	dw FxCallRoutine
	dw FxFixedMode ; Repurposed note delay
	dw FxSetPanning
	dw FxChangeTimbre
	dw FxVolumeSlide
	dw KnownRet ; No pos jump
	dw FxSetVolume
	dw KnownRet ; No pattern break
	dw KnownRet ; No note cut
	dw KnownRet ; This would reset the row timer, and you DEFINITELY don't want that.


; @param hl: Pointer to the channel's order pointer.
; @param b:  Offset into the current patterns.
; @return hl:   Pointer to the channel's period (1, 2, 3) / polynom (4).
; @return c:    New row's instrument/FX byte.
; @return d:    New row's note index.
; @return zero: If set, the note should not be played.
; @destroy e a
ReadRow:
	; Compute the pointer to the current pattern.
	ld a, [wOrderIdx] ; TODO: cache this in a reg across calls?
	add a, [hl]
	ld e, a
	inc hl
	ld a, [hli]
	adc a, 0
	ld d, a
	assert wCH1.order + 2 == wCH1.fxParams
	; Compute the pointer to the current row.
	ld a, [de]
	add a, b
	ld c, a
	inc de
	ld a, [de]
	adc a, 0
	ld d, a
	ld e, c
	; Read the row's ID, and compute the pointer to the actual row.
	ld a, [de]
	ld e, a
	ld a, [wRowCatalogHigh]
	ld d, a
	; Read the row into the channel's data.
	fO_log main_row, "CH\{((hl - wCH1.fxParams) / (wCH2 - wCH1)) + 1\} reading row from $\{de,04$\}: (\{[de + 512],2\}, \{[de + 256],2$\}_\{[de],2$\})"
	ld a, [de]
	ld [hli], a
	inc d
	assert wCH1.fxParams + 1 == wCH1.instrAndFX
	ld a, [de]
	ld [hli], a
	ld c, a
	inc d
	assert wCH1.instrAndFX + 1 == wCH1.note
	ld a, [de]
	runtime_assert a < {d:LAST_NOTE} || a == {d:___}, "Invalid note ID \{a,#\}"
	ld d, a
	; If the row is a rest, don't play it.
	cp ___
	ret z
	ld [hli], a ; Only write the note back if it's not a rest.
	; If the FX is a tone porta or a note delay, don't play the note yet.
	ld a, c
	assert FX_NOTE_DELAY == FX_TONE_PORTA | $04, "Difference between note delay (${x:FX_NOTE_DELAY}) and tone porta (${x:FX_TONE_PORTA}) must be a single bit"
	and $0F & ~$04
	cp FX_TONE_PORTA
	ret


; @param e: The ID of the wave to load.
; @destroy hl e a
LoadWave:
	; Compute a pointer to the wave.
	ld a, e
.waveInA
	ld [hUGE_LoadedWaveID], a
	ld hl, wWaves
	add a, [hl]
	inc hl
	ld h, [hl]
	ld l, a
	adc a, h
	sub l
	ld h, a

	IF !DEF(FORTISSIMO_CH3_KEEP)
		; Temporarily "disconnect" CH3 while loading the wave, to mitigate the DC offset from turning the DAC off.
		ldh a, [rNR51]
		ld e, a
		and ~(AUDTERM_3_LEFT | AUDTERM_3_RIGHT)
		ldh [rNR51], a
	ENDC

	; Load the wave.
	xor a
	ldh [rNR30], a ; Disable CH3's DAC while loading wave RAM.
	FOR OFS, 0, 16
		ld a, [hli]
		ldh [_AUD3WAVERAM + OFS], a
	ENDR
	ld a, AUD3ENA_ON
	ldh [rNR30], a ; Re-enable CH3's DAC.

	IF !DEF(FORTISSIMO_CH3_KEEP)
		ld a, e
		ldh [rNR51], a
	ENDC
	ret


; Starts playing a new note on the channel, writing back its period to the channel's struct.
; The "standard" comes from CH4 encoding its frequency in a special way, whereas channels 1, 2, and 3
; all work (mostly) the same.
; @param a:  ID of the note to play.
; @param c:  LOW(rNRx3)
; @param hl: Pointer to the channel's `.period`.
; @destroy c de hl a
def PlayNewNoteStandard equs "PlayDutyNote.playNewNote"


; @param de: Pointer to the channel's FX params.
; @param c:  The channel's ID (0 for CH1, 1 for CH2, etc.)
; @destroy a bc de hl (potentially)
RunTick0Fx:
	ld a, [de]
	ld b, a
	inc de
	assert wCH1.fxParams + 1 == wCH1.instrAndFX
	ld a, [de]
	and $0F ; Strip instrument bits.
	add a, a ; Each entry in the table is 2 bytes.
	add a, LOW(Tick0Fx)
	ld l, a
	adc a, HIGH(Tick0Fx)
	sub l
	ld h, a
	assert wCH1.instrAndFX + 1 == wCH1.note
	inc de
	; WARNING: `NoteCutTick0Trampoline` assumes that it's jumped to with `a == h`.
	jp hl


; All of the FX functions follow the same calling convention:
; @param de: Pointer to the channel's note byte.
; @param c:  The channel's mask (the CHx_MASK constant).
; @param b:  The FX's parameters.
; @destroy a bc de hl (potentially)

	override No, To
MACRO No ; For "empty" entries in the JR tables.
	ret
	ds 1
ENDM

DEF NB_TO_PRINT = 0
MACRO To
	jr \1
	IF DEF(PRINT_JR_STATS)
		DEF TO_PRINT{d:NB_TO_PRINT} equs STRCAT(STRRPL("\2", "from ", ""), ",\1")
		DEF NB_TO_PRINT += 1
		DEF FROM equs STRCAT("From", STRRPL("\2", "from ", ""), "To\1")
		{FROM}::
		PURGE FROM
	ENDC
ENDM

; FX code that only runs on tick 0.

FxChangeTimbre2: ; These are jumped to by `FxChangeTimbre` below.
.ch1
	ld a, b
	ldh [rNR11], a
	ret
.ch2
	ld a, b
	ldh [rNR21], a
	ret
.ch3
	ld a, b
	swap a ; TODO: optimise this at compile time
	call LoadWave.waveInA
	; We now need to retrigger the channel, since we had to stop it to reload the wave.
	ld hl, wCH3.period + 1
	ld a, [hld] ; It's annoying that these bits are there, but that's how it is.
	dec hl
	assert wCH3.period + 1 - 2 == wCH3.lengthBit
	or [hl]
	or $80
	ldh [rNR34], a
	ret


FxTonePortaSetup:
	runtime_assert c != $08, "Tone porta is not supported on CH4!"
	; Setup portion: get the target period.
	ld a, [de]
	; Compute the target period from the note ID.
	add a, a
	add a, LOW(PeriodTable)
	ld l, a
	adc a, HIGH(PeriodTable)
	sub l
	ld h, a
	ld a, [hli]
	ld b, [hl]
	; Write it.
	ld hl, wCH1.portaTarget - wCH1.note
	add hl, de
	ld [hli], a
	ld [hl], b
KnownRet:
	ret


; This one is slightly out of order so it can `jr FxChangeTimbre2.chX`.

; For CH1 and CH2, this is written as-is to NRx1;
; for CH3, this is the ID of the wave to load;
; for CH4, this is the new LFSR width bit to write to NR43.
FxChangeTimbre:
	; Don't touch the channel if not allowed to.
	ldh a, [hUGE_AllowedChannels]
	and c
	ret z
	; Dispatch.
	rra
	jr c, FxChangeTimbre2.ch1
	rra
	jr c, FxChangeTimbre2.ch2
	rra
	jr c, FxChangeTimbre2.ch3
.ch4
	; Keep the polynom bits, but replace the LFSR width bit.
	runtime_assert (b == 0) || (b == {AUD4POLY_7STEP}), "Invalid timbre change for CH4!"
	ldh a, [rNR43]
	and ~AUD4POLY_7STEP ; Reset the LFSR width bit.
	or b
	ldh [rNR43], a
	ret


FxSetMasterVolume:
	ld a, b ; Read the FX's params.
	ldh [rNR50], a
	ret


FxSetPanning:
	ld a, b ; Read the FX's params.
	ldh [FORTISSIMO_PANNING], a
	ret


; FxChangeTimbre is a bit above.


FxPatternBreak:
	ld a, b
	ld [wForceRow], a
	ret


FxVolumeSlide:
	runtime_assert a != $04, "Volume slide is not supported for CH3!"
	; Don't touch the channel if not allowed to.
	ldh a, [hUGE_AllowedChannels]
	and c
	ret z
	; Compute a pointer to the volume register.
	; Doing it this way, courtesy of @nitro2k01, is smaller and faster.
	; a = 1 for CH1, 2 for CH2, 8 for CH4.
	xor 2 ; 3, 0, A
	inc a ; 4, 1, B
	xor 4 ; 0, 5, F
	add a, LOW(rNR12)
	ld c, a
	; Prepare the FX params.
	ld a, b
	; $F0 appears quite a few times, cache it in a register.
	ld b, $F0
	and b ; "Up" part.
	ld h, a ; "Up" stored in the *upper* byte.
	ld a, b
	swap a ; "Down" part.
	and b
	ld l, a ; "Down" part stored in the *lower* byte.
	; Make volume adjustments.
	ldh a, [c]
	and b ; Keep only the current volume bits.
	sub l
	jr nc, :+
	xor a ; Clamp.
:
	add a, h
	jr nc, :+
	ld a, b ; Clamp.
:
	; TODO: this only needs to apply when a = 0, which is equivalent to clamping... or Z on the `sub`.
	or AUDENV_UP ; Ensure that writing $00 does *not* kill the channel (reduces pops).
.applyVolume
	ldh [c], a ; Yes, this kills the envelope, but we must retrigger, which sounds bad with envelope anyway.
	; Speaking of, retrigger the channel.
	inc c ; LOW(rNRx3)
	inc c ; LOW(rNRx4)
	; CH4 doesn't have a period, so this'll read garbage;
	; however, this is OK, because the lower 3 bits are ignored by the hardware register anyway.
	; We only need to mask off the upper bits which might be set.
	ld hl, wCH1.period + 1 - wCH1.note
	add hl, de ; Go to the period's high byte
	ldh a, [c]
	xor [hl]
	and AUDHIGH_LENGTH_ON | AUDHIGH_RESTART ; Preserve the length bit, and set the "restart" bit (always reads 1).
	xor [hl]
	ldh [c], a
	ret


FxPosJump:
	; Writing to `orderIdx` directly is safe, because it is only read by `ReadRow`,
	; all calls to which happen before any FX processing. (The rows are cached in RAM.)
	ld hl, wOrderIdx
	ld a, b
	ld [hli], a
	; Set the necessary bits to make this non-zero;
	; if a row is already being forced, this keeps it, but will select row 0 otherwise.
	inc hl
	assert wOrderIdx + 2 == wForceRow
	assert LOW(-PATTERN_LENGTH) == $C0 ; Set the corresponding bits.
	ld a, [hl]
	or $C0
	ld [hl], a
	ret


; The jump table is in the middle of the functions so that backwards `jr`s can be used as well as forwards.
Tick0Fx:
	To FxArpeggio, from Tick0Fx
	No porta up
	No porta down
	To FxTonePortaSetup, from Tick0Fx
	To FxResetVibCounter, from Tick0Fx
	To FxSetMasterVolume, from Tick0Fx
	To FxCallRoutine, from Tick0Fx
	No note delay
	To FxSetPanning, from Tick0Fx
	To FxChangeTimbre, from Tick0Fx
	To FxVolumeSlide, from Tick0Fx
	To FxPosJump, from Tick0Fx
	To FxSetVolume, from Tick0Fx
	To FxPatternBreak, from Tick0Fx
	To NoteCutTick0Trampoline, from Tick0Fx
FxSetSpeed:
	ld a, b
	ld [wTicksPerRow], a
	; We want the new tempo to take effect immediately; so, we must reload the timer as well.
	; This is easy to do, since we are on tick 0.
	ld [wRowTimer], a
	ret


FxSetVolume:
	; Don't touch the channel if not allowed to.
	ldh a, [hUGE_AllowedChannels]
	and c
	ret z
	cp hUGE_CH3_MASK
	jr z, .ch3
	; Compute a pointer to the volume register.
	; Doing it this way, courtesy of @nitro2k01, is smaller and faster.
	; a = 1 for CH1, 2 for CH2, 8 for CH4.
	xor 2 ; 3, 0, A
	inc a ; 4, 1, B
	xor 4 ; 0, 5, F
	add a, LOW(rNR12)
	ld c, a
	; FIXME: hUGEDriver preserves envelope bits for pulse channels, but not for CH4;
	;        according to Coffee Bat, the envelope bits should be preserved if B's lower nibble
	;        (post-`swap`) is non-zero. Decide on a behaviour, and implement it.
	ld a, b
	and $0F
	jr nz, .overwriteEnvelope
	ldh a, [c]
	and $0F ; Preserve the envelope bits.
	; TODO: this might end up killing the channel.
.overwriteEnvelope
	or b
	jr FxVolumeSlide.applyVolume ; Apply volume and retrigger channel.

.ch3
	; "Quantize" the more finely grained volume control down to one of 4 values.
	; FIXME: this is not very linear
	ld a, b
	cp 10 << 4
	jr nc, .one
	cp 5 << 4
	jr nc, .two
	and $F0 ; Discard the envelope bits (which are irrelevant to CH3).
	jr z, .done ; Zero maps to zero.
.three:
	ld a, AUD3LEVEL_25
	jr .done
.two:
	ld a, AUD3LEVEL_50
	db $DC ; call c, <ld a, AUD3LEVEL_100>
.one:
	ld a, AUD3LEVEL_100
.done:
	ldh [rAUD3LEVEL], a
	; CH3 doesn't need a retrigger after writing to NR32.
	ret


; Finally, the effects that run the same regardless of first tick or not.

FxArpeggio:
	; `000` is an empty row, even on CH4. Make it do nothing, as it would glitch out on CH4.
	ld a, b
	and a
	ret z
	; Don't touch the channel if not allowed to.
	ldh a, [hUGE_AllowedChannels]
	and c
	ret z
	runtime_assert a != $08, "Arpeggio is not supported on CH4!"
	; Compute the pointer to NRx3, bit twiddling courtesy of @calc84maniac.
	; a = 1 (CH1), 2 (CH2), or 4 (CH3).
	xor $11  ; 10, 13, 15
	add a, c ; 11, 15, 19
	cp LOW(rNR23)
	adc a, c ; 13, 18, 1D
	ld c, a
	; Pick an offset from the base note.
	ld a, [wArpState]
	dec a
	jr z, .noOffset ; arpState == 1
	dec a
	ld a, b ; Read FX params.
	jr z, .useY ; arpState == 2
	swap a
.useY
	and $0F ; Only keep the selected offset.
.noOffset
	; Add the offset (b & $0F) to the base note.
	ld l, a
	ld a, [de]
	add a, l
	; Play this note.
	ld hl, wCH1.period - wCH1.note
	add hl, de
	jp PlayNewNoteStandard


; This is here so that its `jr`s can reach their targets.
FxResetVibCounter:
	ld hl, wCH1.vibratoPrevArg - wCH1.note
	add hl, de
	; If the previous vibrato arg was the same as this one, simply continue it.
	ld a, b
	cp [hl]
	jr z, FxVibrato
	jr PlayNewVib


FxCallRoutine:
	; Read the pointer to the routine, and call it.
	ld hl, wRoutine
	ld a, [hli]
	ld h, [hl]
	ld l, a
	jp hl


; This is a hack to be able to reach `FxNoteCut` from the "tick 0" table.
NoteCutTick0Trampoline:
	; The function we're jumping to computes the current tick count (counting upwards; our row timer
	; instead ticks downwards for performance's sake), and then cuts the note if on the specified tick.
	; Since we are here, we are on tick 0, so that's what we need `a` to be equal to.
	; For performance's but moreso size's sake (we're very space-bound here), we can't afford a
	; `xor a`.
	; Instead, we rely on this function being called with `a == h` (from `RunTick0Fx`), and jumps to
	; a `sub h`, which is then used as the tick count.
	; This is madness, yes, but this is also efficient, and helps compensate the trampoline's
	; performance penalty.
	jp FxNoteCut.computeTick


; And these FX are "continuous" only.

FxPortaUp:
	runtime_assert a != $08, "Porta up is not supported on CH4!"
	; Don't touch the channel if not allowed to.
	ldh a, [hUGE_AllowedChannels]
	and c
	ret z
	; Compute the pointer to NRx3, bit twiddling courtesy of @calc84maniac.
	; a = 1 (CH1), 2 (CH2), or 4 (CH3).
	xor $11  ; 10, 13, 15
	add a, c ; 11, 15, 19
	cp LOW(rNR23)
	adc a, c ; 13, 18, 1D
	ld c, a
	; Cache length bit for writing to NRx4.
	ld hl, wCH1.lengthBit - wCH1.note
	add hl, de
	ld a, [hli]
	assert wCH1.lengthBit + 1 == wCH1.period
	ld e, a
	; Add param to period, writing it back & to NRx3/4.
	ld a, [hl]
	add a, b
	ld [hli], a
	ldh [c], a
	inc c
	ld a, 0
	adc a, [hl]
	ld [hl], a
	or e ; Add the length bit.
	ldh [c], a
	ret


FxPortaDown:
	runtime_assert a != $08, "Porta down is not supported on CH4!"
	; Don't touch the channel if not allowed to.
	ldh a, [hUGE_AllowedChannels]
	and c
	ret z
	; Compute the pointer to NRx3, bit twiddling courtesy of @calc84maniac.
	; a = 1 (CH1), 2 (CH2), or 4 (CH3).
	xor $11  ; 10, 13, 15
	add a, c ; 11, 15, 19
	cp LOW(rNR23)
	adc a, c ; 13, 18, 1D
	ld c, a
	; Cache length bit for writing to NRx4.
	ld hl, wCH1.lengthBit - wCH1.note
	add hl, de
	ld a, [hli]
	assert wCH1.lengthBit + 1 == wCH1.period
	ld e, a
	; Add param to period, writing it back & to NRx3/4.
	ld a, [hl]
	sub b
	ld [hli], a
	ldh [c], a
	inc c
	sbc a, a
	add a, [hl]
	ld [hl], a
	or e ; Add length bit.
	ldh [c], a
	ret


; The jump table is in the middle of the functions so that backwards `jr`s can be used as well as forwards.
ContinuousFx:
	To FxArpeggio, from ContinuousFx
	To FxPortaUp, from ContinuousFx
	To FxPortaDown, from ContinuousFx
	To FxTonePorta, from ContinuousFx
	To FxVibrato, from ContinuousFx
	No set master volume
	To FxCallRoutine, from ContinuousFx
	To FxNoteDelay, from ContinuousFx
	No set panning
	No set duty cycle
	No volume slide
	No pos jump
	No set volume
	No pattern break
	To FxNoteCut, from ContinuousFx
	ret ; No set speed.


; Tone porta is a bit below, so that it's closer to `FxTonePorta2`.


PlayNewVib:
	; Write this vibrato's argument, then.
	ld [hld], a
	assert wCH1.vibratoPrevArg - 1 == wCH1.vibratoState
	; On the first tick, we will underflow the counter, and the direction will flip to "positive".
	xor a
	ld [hld], a ; Write the counter and the direction bit.
	assert wCH1.vibratoState - 1 == wCH1.vibratoOffset
	; Start at no offset.
	ld [hl], a
	; fallthrough

FxVibrato:
	runtime_assert a != $08, "Vibrato is not supported on CH4!"
	; Don't touch the channel if not allowed to.
	ldh a, [hUGE_AllowedChannels]
	and c
	ret z
	; Compute the pointer to NRx3, bit twiddling courtesy of @calc84maniac.
	; a = 1 (CH1), 2 (CH2), or 4 (CH3).
	xor $11  ; 10, 13, 15
	add a, c ; 11, 15, 19
	cp LOW(rNR23)
	adc a, c ; 13, 18, 1D
	ld c, a
	; Tick the vibrato.
	ld hl, wCH1.vibratoState - wCH1.note
	add hl, de
	ld a, [hl]
	sub $10 ; Decrement the counter in the upper 4 bits.
	jr nc, .noUnderflow
	; Reload counter while toggling the direction bit; optimised thanks to @zlago!
	rra ; Shift the direction bit into carry.
	ccf ; Toggle it.
	ld a, b ; Reload the counter.
	; Transfer carry into bit 0.
	rra
	rlca ; This leaves a copy of bit 0 in carry, so the `rra` is not strictly necessary... were it not for `.noUnderflow`.
.noUnderflow
	ld [hld], a ; Write back the new status.
	assert wCH1.vibratoState - 1 == wCH1.vibratoOffset
	; Now, let's modify the offset accordingly.
	rra ; Shift the direction bit into the carry.
	sbc a, a ; $00 if going up, $FF if going down.
	cpl ; $FF if going up, $00 if going down.
	ld e, a
	; Compute the delta.
	ld a, b
	and $0F ; Only keep the lower half of the parameter.
	; Invert it if going down.
	xor e
	sub e ; `sub $FF` is equivalent to `add 1` / `inc a`
	add a, [hl] ; Add the delta to the offset; this should never over- or underflow.
	ld [hld], a ; Write back the new offset.
	; Add the offset to the period.
	assert wCH1.vibratoOffset - 1 == wCH1.period + 1
	ld e, a
	ld d, [hl] ; Read HIGH(period).
	dec hl
	ld a, [hld] ; Read LOW(period).
	assert wCH1.period - 1 == wCH1.lengthBit
	add a, e
	ldh [c], a
	inc c
	ld a, 0
	adc a, d
	or [hl] ; Add the control bits.
	ldh [c], a
	ret


; This is only half of the logic. The other half is in `FxTonePorta2`.
FxTonePorta:
	runtime_assert a != $08, "Tone porta is not supported on CH4!"
	; Don't touch the channel if not allowed to.
	ldh a, [hUGE_AllowedChannels]
	and c
	ret z
	; Compute the pointer to NRx3, bit twiddling courtesy of @calc84maniac.
	; a = 1 (CH1), 2 (CH2), or 4 (CH3).
	xor $11  ; 10, 13, 15
	add a, c ; 11, 15, 19
	cp LOW(rNR23)
	adc a, c ; 13, 18, 1D
	ld c, a
	; Load the target period.
	ld hl, wCH1.portaTarget + 1 - wCH1.note
	add hl, de
	ld a, [hld]
	ld d, a
	ld a, [hld]
	ld e, a
	assert wCH1.period == wCH1.portaTarget - 2
	; TODO: I don't like that `push`, can we do better?
	dec hl
	push hl ; Save the pointer to LOW(period).
	ld a, [hli]
	ld h, [hl]
	; Compute the delta between `current` and `target`.
	sub e
	ld l, a
	ld a, h
	sbc a, d
	ld h, a
	; Move the (signed) delta towards 0.
	add a, a ; What's the sign of `delta`?
	ld a, l
	jr c, FxTonePorta2.goUp ; `current` < `target`
	sub b
	ld l, a
	ld a, h
	sbc a, 0
	jr FxTonePorta2.checkOverflow


FxNoteDelay:
	; Should the note start now?
	ld a, [wRowTimer]
	ld l, a ; How many ticks are remaining.
	ld a, [wTicksPerRow]
	sub l ; How many ticks have elapsed.
	cp b
	ret nz ; Wait until the time is right.
	; All of the "play note" functions expect the note param in d, so let's begin with that.
	; `PlaySomeDutyNote` also expects `hl` to point at the note, so `ld d, [hl]` is as efficient as
	; `ld a, [de] + `ld d, a` plus `ld h, d` to save it.
	ld h, d
	ld l, e
	ld d, [hl] ; Read the note ID.
	; Dispatch.
	bit 3, c
	jp nz, PlayNoiseNote
	bit 2, c
	jr z, PlaySomeDutyNote ; This is more likely to be followed than just CH3, and it's cheaper.
	jp nz, PlayWaveNote ; I just want to annoy disassemblers. :3


FxNoteCut:
	; Should the note be cut now?
	ld a, [wRowTimer]
	ld h, a ; How many ticks are remaining.
	ld a, [wTicksPerRow]
.computeTick ; WARNING: see comments in `NoteCutTick0Trampoline` about register usage.
	sub h ; How many ticks have elapsed.
	sub b
	ret nz ; Wait until the time is right.
	; Make sure to disable the subpattern as well.
	ld hl, wCH1.subPattern - wCH1.note
	add hl, de
	; Set the pointer to NULL. (a = 0 here.)
	ld [hli], a
	ld [hli], a
	; Don't touch the channel if not allowed to.
	ldh a, [hUGE_AllowedChannels]
	and c
	ret z
	; Ok, time to shut the channel down; this snippet optimised by @nitro2k01.
	; We write $08 to NRx2 to mute the channel without turning off the DAC, which makes a "pop".
	cp hUGE_CH3_MASK ; Two comparisons for the price of one!
	jr z, .ch3
	jr nc, .ch4 ; Only CH4 has a mask greater than CH3's.
	; Compute a pointer to NRx2.
	; CH1 = 1, CH2 = 2
	add a, 2 ; 03, 04
	xor $12  ; 11, 16
	inc a    ; 12, 17
	ld c, a
	ld a, AUDENV_UP
	ldh [c],a
	inc c ; NRx3
	inc c ; NRx4
	ld a, AUDHIGH_RESTART
	ldh [c], a
	ret

.ch4
	assert hUGE_CH4_MASK == AUDENV_UP
	ldh [rNR42], a
	ld a, AUDHIGH_RESTART
	ldh [rNR44], a
	ret

.ch3
	; CH3's DAC is instead controlled by NR30 bit 7, so let's turn it off briefly (again, avoiding pops).
	ld hl, rNR30
	ld [hl], l ; This has bit 7 reset.
	ld [hl], h ; This has bit 7 set.
	ret


; We can't fit all of `FxTonePorta` without `jr FxNoteCut` breaking.
; This is the function's second half.
; The split is essentially free because all code paths in `FxTonePorta` eventually jump anyway.
FxTonePorta2:
.goUp
	add a, b
	ld l, a
	ld a, h
	adc a, 0
.checkOverflow
	; If we over/underflowed, clamp at 0.
	jr nc, .noOverflow
	xor a
	ld l, a
.noOverflow
	ld h, a
	; `delta` was originally `current - target`, so `current` should be `delta + target`.
	add hl, de
	; Write it back.
	ld a, l
	ld d, h ; Save this from being overwritten.
	pop hl ; Get back a pointer to LOW(period).
	ld [hli], a
	ldh [c], a
	inc c
	ld a, d
	ld [hld], a
	dec hl
	assert wCH1.period - 1 == wCH1.lengthBit
	or [hl]
	ldh [c], a
	ret


FxFixedMode:
	runtime_assert @b < {LAST_NOTE}, "Fixed mode param must not be higher than {LAST_NOTE}!"
	bit 3, c
	ld a, b
	jp nz, PlayNoiseNote.setFreq
	ld hl, wCH1.period - wCH1.note
	add hl, de
	; Compute the pointer to NRx3, bit twiddling courtesy of @calc84maniac.
	ld a, c	; a = 1 (CH1), 2 (CH2), or 4 (CH3).
	xor $11  ; 10, 13, 15
	add a, c ; 11, 15, 19
	cp LOW(rNR23)
	adc a, c ; 13, 18, 1D
	ld c, a
	ld a, b
	jr PlayNewNoteStandard


; Used by `FxNoteDelay`, hoisted out to save some space in the web of `jr`s that is FX code.
; @param c:  The channel's bit mask in the muted/allowed channel bytes.
; @param d:  The note's ID.
; @param hl: Pointer to the channel's note byte
; @destroy c de hl a
PlaySomeDutyNote:
	; The duty function expects a few more params.
	dec hl
	assert wCH1.note - 1 == wCH1.instrAndFX
	ld e, c ; Transfer the bit mask, since we already have it.
	; We must now compute LOW(rNRx4): $10 for CH1, $15 for CH2.
	srl c ; 00 / 01
	jr z, :+
	set 2, c ; 05
:
	set 4, c ; 10 / 15
	; fallthrough

; @param c:  LOW(rNRx0)
; @param e:  The channel's bit mask in the muted/allowed channel bytes.
; @param d:  The note's ID.
; @param hl: Pointer to the channel's fx/instrument byte.
; @destroy c de hl a
PlayDutyNote:
	; If the channel is inhibited, don't perform any writes.
	ldh a, [hUGE_MutedChannels]
	and e
	ret nz

	; Let's roll!
	push de ; Save the note index for later.
	; First, apply the instrument.
	ld a, [hli]
	inc hl
	assert wCH1.instrAndFX + 2 == wCH1.subPattern
	and $F0 ; Keep the instrument bits.
	jr z, .noInstr
	; Compute the instrument pointer.
	sub $10 ; Instrument IDs are 1-based.
	; Pulse instruments are 6 bytes each, and we currently have the index times 16; scale it down a bit.
	assert DUTY_INSTR_SIZE == 6
	rra ; *8 now.
	rra ; *4 now.
	ld d, a
	rra ; *2 now.
	add a, d ; *2 + *4 = *6, perfect!
	ld d, a
	; If the channel is not inhibited, allow it to be used by FX as well.
	; We can't do this earlier because of register pressure.
	ldh a, [hUGE_AllowedChannels]
	or e
	ldh [hUGE_AllowedChannels], a
	; Resume computing the instrument pointer.
	ld a, [wDutyInstrs]
	add a, d
	ld e, a
	ld a, [wDutyInstrs + 1]
	adc a, 0
	ld d, a
	; Perform the instrument's writes.
	ld a, [de] ; Sweep.
	ldh [c], a
	inc c
	inc de
	ld a, [de] ; Duty & length.
	ldh [c], a
	inc c
	inc de
	ld a, [de] ; Volume & envelope.
	ldh [c], a
	inc de
	ld a, [de] ; Subpattern pointer.
	ld [hli], a
	inc de
	ld a, [de]
	ld [hli], a
	assert wCH1.subPattern + 2 == wCH1.subPatternRow
	inc de
	xor a ; Subpattern row counter.
	ld [hli], a
	assert wCH1.subPatternRow + 1 == wCH1.lengthBit
	ld a, [de] ; NRx4 mask.
	runtime_assert @a & $80, "Instrument without trigger bit!"
	ld [hli], a
.skippedInstr
	assert wCH1.lengthBit + 1 == wCH1.period
	inc c ; Skip NRx2.

	; Next, apply the note.
	pop af ; Retrieve the note ID (from d to a).
	;; NOTE: aliased as `PlayNewNoteStandard`; if modifying this, please check the documentation accordingly.
	; (An alias is used to keep `.noInstr` below as a local label.)
.playNewNote
	; Compute a pointer to the note's period.
	add a, a
	add a, LOW(PeriodTable)
	ld e, a
	adc a, HIGH(PeriodTable)
	sub e
	ld d, a
	; Write it.
	ld a, [de] ; LOW(Period).
	ld [hli], a
	ldh [c], a
	inc c ; Skip NRx3.
	inc de
	ld a, [de] ; HIGH(Period).
	ld [hld], a
	dec hl
	assert wCH1.period - 1 == wCH1.lengthBit
	or [hl] ; OR the "control bits" with the period's high bits.
	ldh [c], a
	res 7, [hl] ; The only "control bit" that should persist is the length enable.
	ret

.noInstr
	inc c ; Skip NRx0.
	inc c ; Skip NRx1.
	inc hl ; Skip subpattern pointer.
	inc hl
	inc hl ; Skip subpattern row counter.
	inc hl ; Skip length bit.
	jr .skippedInstr


; @param d: The channel's note ID.
; @destroy c de hl a
PlayWaveNote:
	; If the channel is inhibited, don't perform any writes.
	ldh a, [hUGE_MutedChannels]
	and hUGE_CH3_MASK
	ret nz

	; First, apply the instrument.
	ld a, [wCH3.instrAndFX]
	and $F0 ; Keep the instrument bits.
	runtime_assert !@zf || [wCH3.lengthBit] & $80 == 0, "CH3 must only be retriggered on instr code path!" ; See the comment further below about killing CH3.
	jr z, .noWaveInstr
	; Compute the instrument pointer.
	sub $10 ; Instrument IDs are 1-based.
	; Wave instruments are 6 bytes each, and we currently have the index times 16; scale it down a bit.
	assert WAVE_INSTR_SIZE == 6
	rra ; *8 now.
	rra ; *4 now.
	ld e, a
	rra ; *2 now.
	add a, e ; *2 + *4 = *6, perfect!
	ld e, a
	ld hl, wWaveInstrs
	ld a, [hli]
	add a, e
	ld e, a
	adc a, [hl]
	sub e
	ld h, a
	ld l, e
	; If the channel is not inhibited, allow it to be used by FX as well.
	ldh a, [hUGE_AllowedChannels]
	or hUGE_CH3_MASK
	ldh [hUGE_AllowedChannels], a
	; Perform the instrument's writes.
	ld a, [hli] ; Length.
	ldh [rNR31], a
	ld a, [hli] ; Volume & envelope.
	ldh [rNR32], a
	ld a, [hli] ; Subpattern pointer.
	ld [wCH3.subPattern], a
	ld a, [hli]
	ld [wCH3.subPattern + 1], a
	xor a ; Subpattern row counter.
	ld [wCH3.subPatternRow], a
	ld a, [hli] ; NRx4 mask.
	runtime_assert @a & $80, "Instrument without trigger bit!"
	ld [wCH3.lengthBit], a
	; Check if a new wave must be loaded.
	ld e, [hl]
	ld a, [hUGE_LoadedWaveID]
	cp e
	call nz, LoadWave
	; Careful—triggering CH3 while it's reading wave RAM can corrupt it.
	; We first kill the channel, and re-enable it, which has it enabled but not playing.
	ld hl, rNR30
	ld [hl], l ; This has bit 7 reset, killing the channel.
	ld [hl], h ; This has bit 7 set, re-enabling the channel.
.noWaveInstr

	; Next, apply the note.
	ld a, d ; Retrieve the note ID.
	; Compute a pointer to the note's period.
	add a, a
	add a, LOW(PeriodTable)
	ld e, a
	adc a, HIGH(PeriodTable)
	sub e
	ld d, a
	; Write it.
	ld hl, wCH3.period
	ld a, [de] ; LOW(Period).
	ld [hli], a
	inc de
	ldh [rNR33], a
	ld a, [de] ; HIGH(Period).
	ld [hld], a
	dec hl
	assert wCH3.period - 1 == wCH3.lengthBit
	or [hl] ; OR the "control bits" with the period's high bits.
	ldh [rNR34], a
	res 7, [hl] ; The only "control bit" that should persist is the length enable.
	ret

; @param d: The channel's note ID.
; @destroy c de hl a
PlayNoiseNote:
	; If the channel is inhibited, don't perform any writes.
	ldh a, [hUGE_MutedChannels]
	and hUGE_CH4_MASK
	ret nz

	; First, apply the instrument.
	ld a, [wCH4.instrAndFX]
	and $F0 ; Keep the instrument bits.
	jr z, .noNoiseInstr
	; Compute the instrument pointer.
	sub $10 ; Instrument IDs are 1-based.
	; Noise instruments are 4 bytes each, and we currently have the index times 16; scale it down a bit.
	; TODO: assert that this is the case!
	rra ; *8 now.
	rra ; *4 now.
	ld e, a
	ld hl, wNoiseInstrs
	ld a, [hli]
	add a, e
	ld e, a
	adc a, [hl]
	sub e
	ld h, a
	ld l, e
	; If the channel is not inhibited, allow it to be used by FX as well.
	ldh a, [hUGE_AllowedChannels]
	or hUGE_CH4_MASK
	ldh [hUGE_AllowedChannels], a
	; Perform the instrument's writes.
	ld a, [hli] ; Volume & envelope.
	ldh [rNR42], a
	ld a, [hli] ; Subpattern pointer.
	ld [wCH4.subPattern], a
	ld a, [hli]
	ld [wCH4.subPattern + 1], a
	xor a ; Subpattern row counter.
	ld [wCH4.subPatternRow], a
	ld a, [hl] ; LFSR width & length bit & length.
	and $3F ; Only keep the length bits.
	ldh [rNR41], a
	; What follows is a somewhat complicated dance that saves 1 cycle over loading from [hl] then
	; ANDing the desired bit twice. Totally worth it, if only because it looks cool af.
	xor [hl] ; Only keep the other two bits (bits 7 and 6, LFSR width and length enable respectively).
	rlca ; Now, LFSR width is in bit 0 and carry, and length enable in bit 7.
	scf
	rra ; Now, LFSR width is in carry, length enable in bit 6, and bit 7 (the retrigger flag) is set.
	ld [wCH4.lengthBit], a
	sbc a, a ; This "broadcasts" the LFSR width to all bits of A.
	and AUD4POLY_7STEP ; Only keep a single one of those, though.
	ld [wCH4.lfsrWidth], a
.noNoiseInstr

	; Next, apply the note.
	ld a, d
.setFreq
	call GetNoisePolynom
	ld hl, wCH4.polynom
	ld [hld], a
	assert wCH4.polynom - 1 == wCH4.lfsrWidth
	runtime_assert (a & $08) == 0, "Polynom \{@a,2$\} has bit 3 set!"
	or [hl] ; The polynom's bit 3 is always reset.
	ldh [rNR43], a
	dec hl
	assert wCH4.lfsrWidth - 1 == wCH4.lengthBit
	ld a, [hl]
	ldh [rNR44], a
	res 7, [hl] ; The only "control bit" that should persist is the length enable.
	ret


; @param a: The note's index (range: 0..=63).
; @return a: The note's "polynom" (what must be written to NR43).
; @destroy e
GetNoisePolynom:
	; Flip the range.
	add 256 - 64
	cpl

	; Formula by RichardULZ:
	; https://docs.google.com/spreadsheets/d/1O9OTAHgLk1SUt972w88uVHp44w7HKEbS#gid=75028951
	; if a > 7 {
	;     let b = (a - 4) / 4;
	;     let c = (a % 4) + 4;
	;     a = c | (b << 4);
	; }

	; If a <= 7 (a < 8), do nothing.
	cp 8
	ret c
	; b = (a - 4) / 4, so b = a / 4 - 1
	ld e, a
	srl e ; / 2, e in 4..=31
	srl e ; / 4, e in 2..=15
	dec e ; e in 1..=14
	; c = (a % 4) + 4, so c = (a & 3) + 4
	and 3
	add a, 4
	; a = c | (b << 4)
	swap e ; This is a 4-bit rotate, but it's OK because e's upper 4 bits were clear.
	or e
	ret


ContinueFx: ; TODO: if this is short enough, swapping it with the other path may allow for a `jr`.
	; Run "continuous" FX.
	ld hl, wCH1.fxParams
	ld c, hUGE_CH1_MASK
	call .runFx
	ld hl, wCH1.lengthBit
	ld c, hUGE_CH1_MASK
	call TickSubpattern

	ld hl, wCH2.fxParams
	ld c, hUGE_CH2_MASK
	call .runFx
	ld hl, wCH2.lengthBit
	ld c, hUGE_CH2_MASK
	call TickSubpattern

	ld hl, wCH3.fxParams
	ld c, hUGE_CH3_MASK
	call .runFx
	ld hl, wCH3.lengthBit
	ld c, hUGE_CH3_MASK
	call TickSubpattern

	ld hl, wCH4.fxParams
	ld c, hUGE_CH4_MASK
	call .runFx
	ld hl, wCH4.lengthBit
	ld c, hUGE_CH4_MASK
	jp TickSubpattern

; @param hl: Pointer to the channel's FX params.
; @param c:  The channel's ID (0 for CH1, 1 for CH2, etc.)
; @destroy a bc de hl (potentially)
.runFx
	ld a, [hli]
	ld b, a
	ld a, [hli]
	assert wCH1.instrAndFX + 1 == wCH1.note
	ld e, l
	ld d, h
	and $0F ; Strip instrument bits.
	add a, a ; Each entry in the table is 2 bytes.
	add a, LOW(ContinuousFx)
	ld l, a
	adc a, HIGH(ContinuousFx)
	sub l
	ld h, a
	jp hl


PeriodTable:
	IF !DEF(HUGETRACKER)
		INCLUDE "hUGE_note_table.inc"
	ELSE
		INCLUDE "include/hUGE_note_table.inc"
	ENDC


PUSHS
SECTION "Music driver RAM", FORTISSIMO_RAM

; While a channel can be safely tinkered with while muted, if wave RAM is modified,
; `hUGE_NO_WAVE` must be written to this variable before unmuting channel 3.
_hUGE_LoadedWaveID::
hUGE_LoadedWaveID:: db ; ID of the wave the driver currently has loaded in RAM.
	DEF hUGE_NO_WAVE equ 100
	EXPORT hUGE_NO_WAVE

wArpState: db ; 1 = No offset, 2 = Use Y, 3 = Use X. Global coounter for continuity across rows.
wRowTimer: db ; How many ticks until switching to the next row.

; Active song "cache".

wTicksPerRow: db ; How many ticks between each row.
wLastPatternIdx: db ; Index of the last pattern in the orders.

wDutyInstrs: dw
wWaveInstrs: dw
wNoiseInstrs: dw

wRoutine: dw

wWaves: dw

wRowCatalogHigh: db
wSubpatRowCatalogHigh: db

; Global variables.

IF DEF(PREVIEW_MODE)
current_order:
ENDC
wOrderIdx: db ; Index into the orders, *in bytes*.
wPatternIdx: db ; Index into the current patterns, with the two high bits set.
wForceRow: db ; If non-zero, will be written (verbatim) to `patternIdx` on the next tick 0, bypassing the increment.

; Individual channels.
	override channel
MACRO channel
	; Pointer to the channel's order; never changes mid-song.
	; (Part of the "song cache", in a way, but per-channel.)
	.order: dw
	; The current row's FX parameters.
	.fxParams: db
	; The current row's instrument (high nibble) and FX ID (low nibble).
	.instrAndFX: db
	; The current row's note.
	.note: db
	.subPattern: dw ; Pointer to the channel's subpattern.
	.subPatternRow: db ; Which row the subpattern is currently in.
	.lengthBit: db ; The upper 2 bits written to NRx4.
	IF (\1) != 4
		; The current "period" (what gets written to NRx3/NRx4).
		; This must be cached for effects like toneporta, which decouple this from the note.
		.period: dw
		UNION
			; The "period" that tone porta slides towards.
			; (Redundant with the "note", but makes the "continuous" FX code faster.)
			.portaTarget: dw
		NEXTU
			.vibratoOffset: db ; How much to add to `period` into NRx3/4.
			.vibratoState: db ; Upper 4 bits count down, bit 0 contains the "direction" (0 = up, 1 = down).
			; If the previous row contained a vibrato, then this contains its arg; if not, the low 4 bits are zero.
			; (This is OK because the vibrato wouldn't be reset if and only if its own "slope" was 0,
			; which makes it have no effect anyway.)
			.vibratoPrevArg: db
		ENDU
	ELSE ; CH4 is a lil' different.
		; The LFSR width bit (as in NR43).
		.lfsrWidth: db
		; The current "polynom" (what gets written to NR43).
		.polynom: db
	ENDC
	; WARNING: If adding any variables here, you may want to add padding to ensure the two `IF` arms above have the same size!
	.end
ENDM

wCH1:  channel 1
wCH2:  channel 2
wCH3:  channel 3
wCH4:  channel 4


SECTION "Music driver HRAM", HRAM

; `hUGE_AllowedChannels` is accessed directly a *lot* in FX code, and the 1-byte save from `ldh`
; helps keeping all the code in `jr` range.
; hUGE_FxParams allows reducing register pressure in TickSubpattern.

hUGE_FxParam: db ; Temporary variable in TickSubpattern.

; As soon as a channel's bit gets set in this variable, the driver will stop updating any of its registers.
; This is useful, for example, for playing sound effects: temporarily "mute" one of the song's channels,
; play the SFX on it, and un-mute it to "release" it back to the driver.
;
; Note that the driver will not touch the channel until a new note plays on it.
;
; IMPORTANT: muting CH3 (the wave channel) requires special attention: if wave RAM is modified, you
;            must set `hUGE_LoadedWaveID` to `hUGE_NO_WAVE` before unmuting the channel.
_hUGE_MutedChannels:: ; C interface.
hUGE_MutedChannels:: db
hUGE_AllowedChannels: db ; Bit mask of which channels the driver is allowed to use.

; The two variables above use these masks.
	DEF hUGE_CH1_MASK equ 1 << 0
	DEF hUGE_CH2_MASK equ 1 << 1
	DEF hUGE_CH3_MASK equ 1 << 2
	DEF hUGE_CH4_MASK equ 1 << 3
	EXPORT hUGE_CH1_MASK, hUGE_CH2_MASK, hUGE_CH3_MASK, hUGE_CH4_MASK

POPS


;; hUGETracker compatibility layer follows.


IF DEF(HUGETRACKER)
	PUSHS
		SECTION "Converted data", WRAM0
		wConvertedNoiseInstrs: ds 15 * 4
		wConvertedWaveInstrs: ds 15 * 6
		wConvertedHeader:
			.tempo: db
			.maxIndex: db
			.instrPtrs: ds 2 * 3
			.routine: dw
			.waves: dw
			.rowCatalogHigh: db
	POPS

	; hUGETracker generates a different "track header", which must be translated so that fO can understand it.
	; @param hl: Pointer to song header.
	hUGE_init::
		; This init routine is not particularly optimised, since the cycle budget in the tracker is much more lenient.

		; Perform what would normally be the "global init".
		xor a
		ldh [hUGE_MutedChannels], a

		; Begin converting the song header from hD's format to fO's.

		ld a, [hli] ; Tempo byte.
		ld [wConvertedHeader.tempo], a

		; Pointer to `order_cnt`.
		ld a, [hli]
		ld e, a
		ld a, [hli]
		ld d, a
		ld a, [de]
		sub 2 ; hD stores the size times two, fO stores the ID of the last one times two.
		ld [wConvertedHeader.maxIndex], a

		; Order pointers.
		; We'll need them later.
		ld c, 4
	.saveOrderPtr
		ld a, [hli]
		ld e, a
		ld a, [hli]
		ld d, a
		push de
		dec c
		jr nz, .saveOrderPtr

		; Duty instruments have the same format, so we can simply point at their table.
		ld a, [hli]
		ld [wConvertedHeader.instrPtrs], a
		ld a, [hli]
		ld [wConvertedHeader.instrPtrs + 1], a
		; All other instrument data needs to be converted, as the subpattern row format is different in hD and fO.
		REPT 2
			ld a, [hli]
			ld e, a
			ld a, [hli]
			ld d, a
			push de
		ENDR

		; Stub out the routine.
		inc hl
		inc hl
		ld a, LOW(KnownRet)
		ld [wConvertedHeader.routine], a
		ld a, HIGH(KnownRet)
		ld [wConvertedHeader.routine + 1], a

		; The waves.
		ld a, [hli]
		ld [wConvertedHeader.waves], a
		ld a, [hli]
		ld [wConvertedHeader.waves + 1], a

		; And finally, the row catalog.
		ld a, HIGH(Catalog)
		ld [wConvertedHeader.rowCatalogHigh], a

		; Convert the instruments.

		pop de
		ld hl, wConvertedNoiseInstrs
		ld c, 15
	.convertNoiseInstr
		; Volume & envelope.
		ld a, [de]
		inc de
		ld [hli], a
		; Subpattern pointer.
		ld a, [de]
		inc de
		ld [hli], a
		ld a, [de]
		inc de
		ld [hli], a
		; A bunch of flags.
		ld a, [de]
		inc de
		ld [hli], a
		; Skip padding.
		inc de
		inc de
		dec c
		jr nz, .convertNoiseInstr
		ld a, LOW(wConvertedNoiseInstrs)
		ld [wConvertedHeader.instrPtrs + 4], a
		ld a, HIGH(wConvertedNoiseInstrs)
		ld [wConvertedHeader.instrPtrs + 4 + 1], a

		pop de
		ld hl, wConvertedWaveInstrs
		ld c, 15
	.convertWaveInstr
		; Length.
		ld a, [de]
		inc de
		ld [hli], a
		; Output level.
		ld a, [de]
		inc de
		ld [hli], a
		; Wave ID (last in fO).
		ld a, [de]
		inc de
		push af
		; Subpattern pointer.
		ld a, [de]
		inc de
		ld [hli], a
		ld a, [de]
		inc de
		ld [hli], a
		; Retrigger bit and length enable.
		ld a, [de]
		inc de
		ld [hli], a
		; Write the wave ID.
		pop af
		swap a ; Multiply by the wave's length.
		ld [hli], a
		dec c
		jr nz, .convertWaveInstr
		ld a, LOW(wConvertedWaveInstrs)
		ld [wConvertedHeader.instrPtrs + 2], a
		ld a, HIGH(wConvertedWaveInstrs)
		ld [wConvertedHeader.instrPtrs + 2 + 1], a

		; And now, load this converted header!
		ld de, wConvertedHeader
		call hUGE_SelectSong
		; One small catch: normally, the pattern tables lie right after the header... but not here!
		FOR N, 4, 0, -1
			pop de
			ld a, e
			ld [wCH{d:N}.order], a
			ld a, d
			ld [wCH{d:N}.order + 1], a
		ENDR
		ret
ENDC

IF DEF(PREVIEW_MODE)
	hUGE_dosound::
		; Check if the tracker requested a change of orders.
		ld a, [next_order]
		and a
		jr z, .noOrderChange
		dec a
		add a, a
		ld [wOrderIdx], a
		xor a
		ld [next_order], a
	.noOrderChange

		; Check if the tracker requested a row break.
		ld a, [row_break]
		and a
		jr z, .noBreak
		dec a
		assert PATTERN_LENGTH == 1 << 6, "Pattern length must be a power of 2"
		or -PATTERN_LENGTH
		ld [wForceRow], a
		; Forcing the row also increases the order index... undo that.
		ld a, [wOrderIdx]
		dec a
		dec a
		ld [wOrderIdx], a
		; Acknowledge the request.
		xor a
		ld [row_break], a
	.noBreak

		call hUGE_TickSound
		db $f4 ; Signal tracker to take a snapshot of audio regs (for VGM export).

		; Convert row info to the format the tracker expects.
		ld a, [wPatternIdx]
		assert PATTERN_LENGTH == 1 << 6, "Pattern length must be a power of 2"
		and PATTERN_LENGTH - 1
		ld [row], a
		db $fd ; Signal tracker to re-read the row index.
		ret

	SECTION "Preview variables", WRAM0

	row: db
	row_break: db
	next_order: db
	loop_order: db ; If non-zero, instead of falling through to the next pattern, loop the current one.

ELIF DEF(HUGETRACKER)
	_hUGE_dosound::
	hUGE_dosound::
		jp hUGE_TickSound
ENDC


	override print_pair, print_stats
MACRO print_pair
	PRINTLN STRFMT("%s -> %s = %d", "\1", "\2", \2 - From\1To\2)
ENDM
MACRO print_stats
	FOR I, NB_TO_PRINT
		print_pair {TO_PRINT{d:I}}
	ENDR
ENDM
	print_stats
