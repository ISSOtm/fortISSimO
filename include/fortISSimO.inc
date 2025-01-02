; This file is to be included by generated songs as well.

IF !DEF(FORTISSIMO_INC) ; Simple include guard, but also used by fO to check that the correct include file is provided.
def FORTISSIMO_INC equ 1

def PATTERN_LENGTH equ 64 ; How many rows in a pattern.
def ROW_SIZE equ 3 ; How many bytes per row.
def ORDER_WIDTH equ 2 ; How many bytes in each order entry.

def DUTY_INSTR_SIZE equ 6
def WAVE_INSTR_SIZE equ 6
def NOISE_INSTR_SIZE equ 4

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

; hUGETracker compatibility layer.
MACRO dn
    DEF STEAL_FLAG = (\2) >> 4
    DEF INSTR = (\2) & $0F
    DEF FX = HIGH(\3)
    DEF FX_ARG = LOW(\3)

    IF !DEF(Catalog)
        ; First call, set the base addresses.
        PUSHS
        SECTION FRAGMENT "Row first bytes", ROMX[$7D00]
            Catalog:: ; Needs to be accessed by fO.
        SECTION FRAGMENT "Row second bytes", ROMX[Catalog + 256]
        SECTION FRAGMENT "Row third bytes", ROMX[Catalog + 256 * 2]
        POPS
    ENDC

    IF FX == FX_POS_JUMP
        DEF FIRST_BYTE = (FX_ARG - 1 - 1) * 2
    ELIF FX == FX_SET_VOLUME
        DEF FIRST_BYTE = (FX_ARG >> 4) | (FX_ARG & $0F) << 4
        ; If we're going to set the envelope, avoid killing the channel.
        IF FIRST_BYTE & $0F != 0 && FIRST_BYTE & $F8 == 0
            DEF FIRST_BYTE = $08
        ENDC
    ELIF FX == FX_PATTERN_BRK
        DEF FIRST_BYTE = (FX_ARG - 1) | -PATTERN_LENGTH
    ELSE
        DEF FIRST_BYTE = FX_ARG
    ENDC
    DEF SECOND_BYTE = INSTR << 4 | FX
    DEF THIRD_BYTE = STEAL_FLAG << 7 | (\1)

    DEF LABEL equs "Row_{X:FIRST_BYTE}_{X:SECOND_BYTE}_{X:THIRD_BYTE}"
    IF !DEF({LABEL}) ; Only emit each row once.
        PUSHS
        SECTION FRAGMENT "Row first bytes", ROMX
        {LABEL}:
            db FIRST_BYTE
        SECTION FRAGMENT "Row second bytes", ROMX
            db SECOND_BYTE
        SECTION FRAGMENT "Row third bytes", ROMX
            db THIRD_BYTE
        POPS
    ENDC
        db LABEL - Catalog
    PURGE LABEL
ENDM
ENDC
