
INCLUDE "config.inc"
INCLUDE "constants.inc"

INCLUDE "notes.inc"

INCLUDE "driver_mem.asm"
INCLUDE "driver.asm" ; Make sure to define this last, so music data can go in the same section


; Stuff for user definitions

row: MACRO ;; (note, instr, effect)
    db LOW(\3)
    db ((HIGH(\3) << 4) | (\2))
    db \1
ENDM
