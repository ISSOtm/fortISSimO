
define_with_default: MACRO
    IF !DEF(\1)
\1 equs "\2"
    ENDC
ENDM


    define_with_default hUGE_CODE_SECTION_DECL, ROM0
    define_with_default hUGE_RAM_SECTION_DECL,  WRAM0

    ; hardware.inc constants
    define_with_default rNR10, $FF10
    define_with_default rNR11, $FF11
    define_with_default rNR12, $FF12
    define_with_default rNR13, $FF13
    define_with_default rNR14, $FF14
    define_with_default rNR20, $FF15 ; Not mapped
    define_with_default rNR21, $FF16
    define_with_default rNR22, $FF17
    define_with_default rNR23, $FF18
    define_with_default rNR24, $FF19
    define_with_default rNR30, $FF1A
    define_with_default rNR31, $FF1B
    define_with_default rNR32, $FF1C
    define_with_default rNR33, $FF1D
    define_with_default rNR34, $FF1E
    define_with_default rNR40, $FF1F ; Not mapped
    define_with_default rNR41, $FF20
    define_with_default rNR42, $FF21
    define_with_default rNR43, $FF22
    define_with_default rNR44, $FF23
    define_with_default rNR50, $FF24
    define_with_default rNR51, $FF25
    define_with_default rNR52, $FF26
