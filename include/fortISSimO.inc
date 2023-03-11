; This file is to be included by generated songs as well.

DEF PATTERN_LENGTH equ 64 ; How many rows in a pattern.
DEF ROW_SIZE equ 3 ; How many bytes per row.
DEF ORDER_WIDTH equ 2 ; How many bytes in each order entry.

MACRO row ; Usage: `row note, instr, effect ID, effect param`
    ; For practicality, the `row`'s arguments are meant to mirror what was in the tracker.
    ; However, some transformations can be made at compile time, which simplifies the driver, makes
    ; it faster, and most importantly, "loosens" the `jr` tangle hell that is the FX code.
    DEF NOTE = (\1)
    DEF INSTR = (\2) & $0F
    DEF FX = ($\3)
    DEF FX_ARG = ($\4)
    assert FX_ARG == LOW(FX_ARG), "Bad FX arg {FX_ARG}"

    ; Some FX-specific transformations.
    ; TODO: at least do some `assert`s where relevant
    IF FX == FX_ARPEGGIO
    ELIF FX == FX_PORTA_UP
    ELIF FX == FX_PORTA_DOWN
    ELIF FX == FX_TONE_PORTA
    ELIF FX == FX_VIBRATO
    ELIF FX == FX_MASTER_VOL
    ELIF FX == FX_ROUTINE
    ELIF FX == FX_NOTE_DELAY
        assert NOTE < LAST_NOTE, "Cannot use note delay without a note!"
    ELIF FX == FX_PANNING
    ELIF FX == FX_DUTY_CYCLE
    ELIF FX == FX_VOL_SLIDE
    ELIF FX == FX_POS_JUMP
        ; This is 1-based in the tracker. Converting to `orderIdx` format requires subtracting 1.
        def FX_ARG -= 1
        ; In addition, this sets `wForceRow`, which causes the order index to be advanced; compensate for that as well.
        def FX_ARG -= 1
        ; Convert to "order index" format.
        def FX_ARG *= 2
    ELIF FX == FX_SET_VOLUME
        def FX_ARG = LOW(FX_ARG << 4) | FX_ARG >> 4 ; Swap the two nibbles to match NRx2 format.
        IF FX_ARG & $0F != 0 && FX_ARG & $F8 == 0
            ; We would kill the channel! Let's mute it but avoid that.
            def FX_ARG = $08 ; Set the volume to 0, but keep the channel alive to avoid a pop.
        ENDC
    ELIF FX == FX_PATTERN_BRK
        assert FX_ARG != 0, "Pattern break is supposed to be 1-based!"
        def FX_ARG -= 1
        assert PATTERN_LENGTH == 1 << 6, "Pattern length must be a power of 2"
        def FX_ARG |= -PATTERN_LENGTH ; Convert to `forceRow` format.
    ELIF FX == FX_NOTE_CUT
    ELIF FX == FX_SET_SPEED
    ELSE
        FAIL "Unknown FX type {FX}"
    ENDC

    ; And finally, the raw binary data.
    db FX_ARG, INSTR << 4 | FX, NOTE
ENDM

MACRO sub_row
    DEF STEAL_FLAG = ((\2) & $10) >> 4
    IF STRCMP("\1", "___") == 0
        DEF NOTE = ___
    ELSE
        DEF NOTE = ((LAST_NOTE / 2) \1)
    ENDC
    row NOTE << 1 | STEAL_FLAG, \2 & $F, \3,\4
ENDM

rsreset
def C_3 rb
def C#3 rb
def D_3 rb
def D#3 rb
def E_3 rb
def F_3 rb
def F#3 rb
def G_3 rb
def G#3 rb
def A_3 rb
def A#3 rb
def B_3 rb
def C_4 rb
def C#4 rb
def D_4 rb
def D#4 rb
def E_4 rb
def F_4 rb
def F#4 rb
def G_4 rb
def G#4 rb
def A_4 rb
def A#4 rb
def B_4 rb
def C_5 rb
def C#5 rb
def D_5 rb
def D#5 rb
def E_5 rb
def F_5 rb
def F#5 rb
def G_5 rb
def G#5 rb
def A_5 rb
def A#5 rb
def B_5 rb
def C_6 rb
def C#6 rb
def D_6 rb
def D#6 rb
def E_6 rb
def F_6 rb
def F#6 rb
def G_6 rb
def G#6 rb
def A_6 rb
def A#6 rb
def B_6 rb
def C_7 rb
def C#7 rb
def D_7 rb
def D#7 rb
def E_7 rb
def F_7 rb
def F#7 rb
def G_7 rb
def G#7 rb
def A_7 rb
def A#7 rb
def B_7 rb
def C_8 rb
def C#8 rb
def D_8 rb
def D#8 rb
def E_8 rb
def F_8 rb
def F#8 rb
def G_8 rb
def G#8 rb
def A_8 rb
def A#8 rb
def B_8 rb
def LAST_NOTE rb
def ___ equ 90 ; the default "no note" value

rsreset
def FX_ARPEGGIO rb
def FX_PORTA_UP rb
def FX_PORTA_DOWN rb
def FX_TONE_PORTA rb
def FX_VIBRATO rb
def FX_MASTER_VOL rb
def FX_ROUTINE rb
def FX_NOTE_DELAY rb
def FX_PANNING rb
def FX_DUTY_CYCLE rb
def FX_VOL_SLIDE rb
def FX_POS_JUMP rb
def FX_SET_VOLUME rb
def FX_PATTERN_BRK rb
def FX_NOTE_CUT rb
def FX_SET_SPEED rb
