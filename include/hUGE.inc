; This file is to be included by generated songs as well.

MACRO dn ;; (note, instr, effect)
    db (\1 | ((\2 & %00010000) << 3))
    db (((\2 << 4) & $FF) | (\3 >> 8))
    db LOW(\3)
ENDM

def C_3       equ 0
def C#3       equ 1
def D_3       equ 2
def D#3       equ 3
def E_3       equ 4
def F_3       equ 5
def F#3       equ 6
def G_3       equ 7
def G#3       equ 8
def A_3       equ 9
def A#3       equ 10
def B_3       equ 11
def C_4       equ 12
def C#4       equ 13
def D_4       equ 14
def D#4       equ 15
def E_4       equ 16
def F_4       equ 17
def F#4       equ 18
def G_4       equ 19
def G#4       equ 20
def A_4       equ 21
def A#4       equ 22
def B_4       equ 23
def C_5       equ 24
def C#5       equ 25
def D_5       equ 26
def D#5       equ 27
def E_5       equ 28
def F_5       equ 29
def F#5       equ 30
def G_5       equ 31
def G#5       equ 32
def A_5       equ 33
def A#5       equ 34
def B_5       equ 35
def C_6       equ 36
def C#6       equ 37
def D_6       equ 38
def D#6       equ 39
def E_6       equ 40
def F_6       equ 41
def F#6       equ 42
def G_6       equ 43
def G#6       equ 44
def A_6       equ 45
def A#6       equ 46
def B_6       equ 47
def C_7       equ 48
def C#7       equ 49
def D_7       equ 50
def D#7       equ 51
def E_7       equ 52
def F_7       equ 53
def F#7       equ 54
def G_7       equ 55
def G#7       equ 56
def A_7       equ 57
def A#7       equ 58
def B_7       equ 59
def C_8       equ 60
def C#8       equ 61
def D_8       equ 62
def D#8       equ 63
def E_8       equ 64
def F_8       equ 65
def F#8       equ 66
def G_8       equ 67
def G#8       equ 68
def A_8       equ 69
def A#8       equ 70
def B_8       equ 71
def LAST_NOTE equ 72
def ___       equ 90 ; the default "no note" value

def FX_ARPEGGIO    equ $0
def FX_PORTA_UP    equ $1
def FX_PORTA_DOWN  equ $2
def FX_TONE_PORTA  equ $3
def FX_VIBRATO     equ $4
def FX_MASTER_VOL  equ $5
def FX_ROUTINE     equ $6
def FX_NOTE_DELAY  equ $7
def FX_PANNING     equ $8
def FX_DUTY_CYCLE  equ $9
def FX_VOL_SLIDE   equ $A
def FX_POS_JUMP    equ $B
def FX_SET_VOLUME  equ $C
def FX_PATTERN_BRK equ $D
def FX_NOTE_CUT    equ $E
def FX_SET_SPEED   equ $F
