
SECTION "hUGE driver code", hUGE_CODE_SECTION_DECL

; Begin playing a song
; @param de A pointer to the song that should be played
; @return a 1
; @destroy bc
hUGE_StartSong::
    ; Prevent playback while we tamper with the state
    xor a
    ld [whUGE_Enabled], a

    ; Read tempo
    ld a, [de]
    inc de
    ld [whUGE_Tempo], a

    ld hl, whUGE_CH1OrderPtr
    call .initChannel
    ld hl, whUGE_CH2OrderPtr
    call .initChannel
    ld hl, whUGE_CH3OrderPtr
    call .initChannel
    ld hl, whUGE_CH4OrderPtr
    call .initChannel

    ; Init APU regs
    ld a, $80
    ldh [rNR52], a
    ld a, $FF
    ldh [rNR51], a
    ld a, $77
    ldh [rNR50], a

    ; Schedule next playback immediately
    ld a, 1
    ld [whUGE_RemainingTicks], a

    ; Re-enable playback
    ; ld a, 1
    ld [whUGE_Enabled], a
    ret

.initChannel
    ; Copy order table ptr
    ld a, [de]
    ld c, a
    inc de
    ld [hli], a
    ld a, [de]
    ld b, a
    inc de
    ld [hli], a
    ; Init row num (will be 0 after 1st increment)
    ld a, -3
    ld [hli], a
    ; Init order index
    xor a
    ld [hli], a
    inc hl ; Skip instrument
    inc hl ; Skip note
    ; Write instrument ptr
    ld a, [bc] ; Read nb of orders
    add a, a
    scf
    adc a, c
    ld [hli], a
    ld a, b
    adc a, 0
    ld [hli], a
    ret


hUGE_TickSound::
    ld a, [whUGE_Enabled]
    and a
    ret z

    ld hl, whUGE_RemainingTicks
    dec [hl]
    jr nz, .noNewNote
    ; Reload tempo
    dec hl
    ld a, [hli]
    ld [hli], a

    ;; Play notes
    ; ld hl, whUGE_CH1OrderPtr
    ld a, LOW(rNR12)
    ld [whUGE_CurChanEnvPtr], a
    ld c, a ; ld c, LOW(rNR12)
    call hUGE_TickChannel
    ld hl, whUGE_CH2OrderPtr
    ld a, LOW(rNR22)
    ld [whUGE_CurChanEnvPtr], a
    ld c, a ; ld c, LOW(rNR22)
    call hUGE_TickChannel
    ld hl, whUGE_CH3OrderPtr
    ld a, LOW(rNR32)
    ld [whUGE_CurChanEnvPtr], a
    ld c, a ; ld c, LOW(rNR32)
    call hUGE_TickChannel
    ld hl, whUGE_CH4OrderPtr
    ld a, LOW(rNR42)
    ld [whUGE_CurChanEnvPtr], a
    ld c, LOW(rNR43)
    call hUGE_TickChannel
.noNewNote

    ; Process effects "update"
    ld a, LOW(rNR42)
    ld hl, whUGE_CH4FX
    call .fxUpdate
    ld a, LOW(rNR32)
    ld hl, whUGE_CH3FX
    call .fxUpdate
    ld a, LOW(rNR22)
    ld hl, whUGE_CH2FX
    call .fxUpdate
    ld a, LOW(rNR12)
    ld hl, whUGE_CH1FX
    ; fallthrough
.fxUpdate
    ld c, a
    ld [whUGE_CurChanEnvPtr], a
    ld a, [hli] ; Read the FX number
    rra ; Is bit 0 set?
    ret c ; Return if no FX
    rla ; Restore value
    add a, LOW(.fxTable)
    ld e, a
    adc a, HIGH(.fxTable)
    sub e
    ld d, a
    push de
    inc hl ; Skip FX buffer, since the FX param tends to get used often
    ret

.fxTable
    jr .noMoreFX ; NYI .fx_arpeggio
    jr .noMoreFX ; NYI .fx_slideUp
    jr .noMoreFX ; NYI .fx_slideDown
    jr .noMoreFX ; NYI .fx_toneporta
    jr .noMoreFX ; NYI .fx_vibrato
    jr .noMoreFX ; jr .fx_setMasterVolume ; Does not update
    jr .fx_callRoutine
    ; jr .fx_noteDelay
    ; Panning and duty don't reach here, so use their space for 4 free bytes
    dec [hl]
    ret nz
    nop ; TODO
    nop
    jr .fx_noteDelay
    jr .fx_volSlide
    jr .noMoreFX ; Free slot
    nop ; jr .fx_setVolume ; Does not update
    nop
    jr .noMoreFX ; Free slot
    ; jr .fx_noteCut
    ; jr .fx_SetSpeed ; Does not update

.fx_noteCut ; No need for a `jr` for this one
    dec [hl]
    ret nz
    ; Write 0 to NRx2 to kill the channel
    xor a
    ldh [c], a
    dec hl ; Skip FX params
    dec hl ; Skip FX buffer
    ld [hl], 1
    ret

.fx_callRoutine
    push hl
    ld h, 1
    call hUGE_CallUserRoutine
    pop hl
    ret nc
    jr .noMoreFX

.fx_noteDelay
    ret

.fx_volSlide
    ; Add a signed 5-bit offset to the current volume
    ld a, [hld] ; Get params
    dec [hl]
    ret nz
    ; Reload counter
    ld b, a
    and %111
    ld [hld], a
    dec hl ; Skip FX number
    ld a, [hl] ; Get current volume (low 4 bits reset)
    rrca
    add a, b ; Add signed 5-bit offset
    and $F8 ; Clear low 3 bits so they don't interfere
    add a, a
    ; If result was negative (due to overflow), the FX is done
    jr nc, hUGE_SetChannelVolume
    inc hl ; Skip volume
    ld [hl], 1
    ret

    ; TODO: get rid of this
.noMoreFX
    ld b, b

; @param hl A pointer to the channel's volume byte
; @param c The low byte of a pointer to NRx2
; @param a The value to set the volume to, **MUST** have low 4 bits reset
; @destroy b
hUGE_SetChannelVolume:
    ld [hld], a
    bit 3, c ; Out of all NRx2, this is only set for NR32
    jr z, .notCH3
    and $C0 ; Keep only the upper 2 bits
    ; Convert to NR32 encoding
    ld b, a
    rrca
    xor b
    ; Bit 7 will be ignored by the hardware
.notCH3
    ldh [c], a ; Write that to NRx2
    set 7, [hl] ; Get the note to retrigger
    jp hUGE_PlayNote ; Retrigger the note to take the volume change into account

; @param a The ID of the routine to call
; @param h Even on 1st call, odd on "updates", including during 1st tick!
hUGE_CallUserRoutine:
    add a, LOW(hUGE_UserRoutines)
    ld l, a
    adc a, HIGH(hUGE_UserRoutines)
    sub l
    rr h ; Transfer bit 0 of H to carry
    ld h, a
    ld a, [hli]
    ld h, [hl]
    ld l, a
    jp hl


hUGE_ChannelJump:
    dec hl
    ; Write new order index
    ld a, b
    ld [hli], a
    ; Write new row index
    ld a, [whUGE_FXParams]
    ld [hli], a
    dec hl

; @param hl Pointer to the channel's data
; @param c Pointer to the first register the instrument will write to
hUGE_TickChannel:
    ; Read order ptr
    ld a, [hli]
    ld e, a
    ld a, [hli]
    ld d, a
    ; Increase row index
    ld a, 3
    add a, [hl]
    ; Check if we need to wrap
    cp PATTERN_LENGTH * 3
    jr c, .samePattern
    xor a
.samePattern
    ld [hli], a
    ld b, a ; Save this for later

    jr c, .noCarry
    inc [hl]
    ld a, [de] ; Read nb of orders
    sub [hl] ; Check if we need to wrap
    jr c, .noCarry
    ld [hl], a ; Apply wrap
.noCarry
    inc de ; Skip nb of orders

    ; Compute ptr to current row in pattern
    ld a, [hli] ; Read order index
    add a, a ; FIXME: assumes order tables are at most 128 orders long
    add a, e
    ld e, a
    adc a, d
    sub e
    ld d, a
    ; Read ptr to B-th pattern
    ld a, [de]
    add a, b
    ld b, a
    inc de
    ld a, [de]
    adc a, 0
    ld d, a
    ld e, b

    ; Read effect params
    ld a, [de]
    inc de
    ld [whUGE_FXParams], a
    ; Read effect + instrument
    ld a, [de]
    inc de
    ld b, a
    ld [hli], a
    ; Read note byte
    ld a, [de]
    cp NOTE_JUMP
    jr z, hUGE_ChannelJump
    ld [whUGE_CurChanNote], a
    ld [hli], a

    ; Read ptr to instrument translation table
    ld a, [hli]
    ld e, a
    ld a, [hli]
    ld d, a

    ; Reset "restart" bit of NRx4 mask
    res 7, [hl]

    ; Compute instrument ptr
    ld a, b
    and $0F ; Mask out other bits
    call nz, hUGE_LoadInstrument
    ld a, [hli]
    ld [whUGE_NRx4Mask], a
    inc hl ; Skip volume

    ; Do effect's first tick
    ld a, b
    and $F0
    jr nz, .doFX
    ; Are arguments 0 as well?
    ld a, [whUGE_FXParams]
    and a
    jp z, .noFX
    xor a ; Restore arpeggio ID
.doFX
    ; Get ID *2
    rra
    rra
    rra
    ld [hli], a
    add a, LOW(.fxTable)
    ld e, a
    adc a, HIGH(.fxTable)
    sub e
    ld d, a
    ld a, [whUGE_FXParams] ; Read this now because most FX use it right away
    push de
    ret


; Each routine gets its params in A
; Some value to put in "param working memory" should be returned in A
; HL must be preserved
.fxTable
    jr .doneWithFX ; NYI .fx_arpeggio
    jr .doneWithFX ; NYI .fx_portaUp
    jr .doneWithFX ; NYI .fx_portaDown
    jr .doneWithFX ; NYI .fx_toneporta
    jr .doneWithFX ; NYI .fx_vibrato
    jr .fx_setMasterVolume
    jr .fx_callRoutine
    jr .fx_noteDelay
    jr .fx_setPan
    jr .fx_setDuty
    jr .fx_volSlide
    jr .doneWithFX ; Free slot
    jr .fx_setVolume
    jr .doneWithFX ; Free slot
    jr .doneWithFX ; jr .fx_noteCut Does not do any init
    ; jr .fx_setSpeed

.fx_setSpeed ; No need for a `jr` for this one
    ld [whUGE_Tempo], a
    jr .noMoreFX

.fx_setMasterVolume
    ldh [rNR51], a
    jr .noMoreFX

.fx_callRoutine
    push hl
    ld h, 0
    call hUGE_CallUserRoutine
    pop hl
    jr c, .noMoreFX
    jr .doneWithFX

.fx_noteDelay
    ; Cancel playing the note
    ld a, [whUGE_CurChanNote]
    ld b, a
    ld a, LAST_NOTE
    ld [whUGE_CurChanNote], a
    ; The note will be played back later
    ld a, b
    jr .doneWithFX

.fx_setPan
    ldh [rNR50], a
    jr .noMoreFX

.fx_setDuty
    ld b, a
    ld a, [whUGE_CurChanEnvPtr]
    dec c
    ld c, a
    ld a, b
    ldh [c], a
    jr .noMoreFX

.fx_volSlide
    ; Schedule effect to happen on next tick
    ld a, 2
    jr .doneWithFX

.fx_setVolume
    ; TODO: take the instrument's envelope into account?
    ld b, a
    ld a, [whUGE_CurChanEnvPtr]
    ld c, a
    ld a, b
    ldh [c], a
    ; jr .noMoreFX


.noMoreFX
    dec hl
.noFX
    ld a, 1
    ld [hli], a
    ; FX storage doesn't matter, write a dummy value there
.doneWithFX
    ; Write FX storage
    ld [hli], a
    ; Write FX params
    ld a, [whUGE_FXParams]
    ld [hli], a

    ; Play the channel's note
    ld a, [whUGE_CurChanNote]
    cp LAST_NOTE
    ret nc
    ; Fallthrough


; @param a The ID of the note to play
hUGE_PlayNote:
    add a, a
    add a, LOW(hUGE_NoteTable)
    ld e, a
    adc a, HIGH(hUGE_NoteTable)
    sub e
    ld d, a
    ; Read period
    ld a, [de]
    ld b, a
    inc de
    ld a, [de]
    ld d, a

    ; Get ptr to NRx3
    ld a, [whUGE_CurChanEnvPtr]
    inc a
    ld c, a
    cp LOW(rNR43)
    jr z, .ch4

    ld a, b
    ldh [c], a
    inc c
    ld a, [whUGE_NRx4Mask]
    or d
    ldh [c], a
    ret

.ch4
    ; Quantize the note by turning it into a sort of "scientific notation"
    ; e = shift amount
    ; db = Frequency, shifted right until it's only 3 bits
    ld e, -3
    ; First, enforce working on a single byte for efficiency
    ld a, d
    and %111
    jr z, .emptyHighByte
    ; Shift right by 5
    xor b
    and %1111
    xor b
    swap a ; This clears carry
    rra ; Shift right one more time
    ld b, a
    ld e, -3 + 5
.emptyHighByte
    ; b = Frequency
    ; Shift right until only 3 significants bits remain
.shiftFreqRight
    ld a, b
    and ~%111
    jr z, .done
    srl b
    inc e
    jr .shiftFreqRight
.done
    swap e
    ldh a, [c] ; Keep length bit
    and %1000
    or b
    or e
    ldh [c], a
    ld a, [whUGE_NRx4Mask]
    ldh [rNR44], a
    ret


; Loads an instrument into a channel's hardware regs
; @param a The index of the instrument to use (starting at 1)
; @param de A pointer to the channel's instrument palette
; @param hl A pointer to the channel's NRx4 mask
; @param c A pointer to the highest IO reg to write to
; @param whUGE_CurChanEnvPtr The low byte of the pointer to NRx2
; @destroy a c de
hUGE_LoadInstrument:
    dec a
    ; Index into translation table
    add a, e
    ld e, a
    adc a, d
    sub e
    ld d, a
    ; Read global instrument ID
    ld a, [de]
    ; Compute ptr to that instrument
    ; FIXME: limits the number of instruments to 64
    add a, a
    add a, a
    add a, LOW(hUGE_Instruments)
    ld e, a
    adc a, HIGH(hUGE_Instruments)
    sub e
    ld d, a

    ; Read NRx4 mask
    ld a, [de]
    inc de
    ld [hli], a
    ; Write last three bytes to hardware regs
    ld a, [de]
    inc de
    ldh [c], a
    dec c
    ld a, [de]
    inc de
    ldh [c], a
    dec c
    ld a, c
    cp LOW(rNR30)
    ld a, [de]
    call z, .loadWave ; This works a tad differently for CH3
    ldh [c], a

    ld a, [whUGE_CurChanEnvPtr]
    ld c, a
    ldh a, [c]
    jr nz, .notCH3
    ; Turn the 2-bit value into the same format as other channels
    add a, a
    and $C0
    ld b, a
    add a, a
    xor b
    ld b, a
    rrca
    rrca
.notCH3
    and $F0
    ld [hld], a
    ret

; @return Z Set
.loadWave
    push hl
    ; Compute ptr to wave
    ; FIXME: limits the number of waves to 16
    add a, LOW(hUGE_Waves)
    ld l, a
    adc a, HIGH(hUGE_Waves)
    sub e
    ld h, a

    ; Kill CH3 while we load the wave
    xor a
    ldh [c], a
hUGE_TARGET = $FF30 ; Wave RAM
REPT 16
    ld a, [hli]
    ldh [hUGE_TARGET], a
hUGE_TARGET = hUGE_TARGET + 1
ENDR
PURGE hUGE_TARGET
    pop hl

    ; Return back to main code, enabling CH3 again
    ld c, LOW(rNR30)
    ld a, $80
    ret


hUGE_NoteTable:
INCLUDE "note_table.inc"
