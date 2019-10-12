
SECTION "hUGE Driver memory", hUGE_RAM_SECTION_DECL

; If this is zero, then `hUGE_TickSound` will do nothing
; This is basically a mutex so the driver does not process an inconsistent state
whUGE_Enabled:
    db

; Each how many ticks a new note will be played
; For example, if this is set to 1, a new note will play every time `hUGE_TickSound` is called
whUGE_Tempo:
    db
; How many ticks until the next note is played
whUGE_RemainingTicks:
    db

hUGE_channel: MACRO
whUGE_CH\1OrderPtr:
    dw
; Which row we are on
whUGE_CH\1RowNum:
    db
whUGE_CH\1OrderIndex:
    db
; Which note the channel is currently playing
whUGE_CH\1Instrument:
    db
; Which note the channel is currently playing
whUGE_CH\1Note:
    db
; A pointer to the 15 instruments the channel will use
whUGE_CH\1InstrPalettePtr:
    dw
; A mask to be applied to rNRx4
whUGE_CH\1NRx4Mask:
    db
; Which effect is currently active, times 2. Bit 0 is set if there is no FX
whUGE_CH\1FX:
    db
; Some byte the FX can use as work RAM
whUGE_CH\1FXBuf:
    db
; The FX's params
whUGE_CH\1FXParams:
    db
ENDM
    hUGE_channel 1
    hUGE_channel 2
    hUGE_channel 3
    hUGE_channel 4
PURGE hUGE_channel

;; Temporary memory for processing

whUGE_FXParams:
    db
whUGE_NRx4Mask:
    db
; The note that the channel should play
whUGE_CurChanNote:
    db
; The low byte of the channel's envelope ptr
whUGE_CurChanEnvPtr:
    db
