    org $700-16

emu_hook equ $1ff
XEX_LOAD equ 1

RUNAD    equ $2e0
INITAD   equ $2e2
COLBK    equ 712

    dta $96
    dta $02

    dta < 128 / 16
    dta > 128 / 16

    dta <128
    dta >128

    dta 0
    dta 0
    dta 0
    dta 0
    dta 0
    dta 0
    dta 0
    dta 0
    dta 0
    dta 0

start:
    dta 0
    dta 1
    dta <start
    dta >start
    dta <do_nothing
    dta >do_nothing

start2:
    lda #<do_nothing
    sta RUNAD
    lda #>do_nothing
    sta RUNAD+1
loop:
    jsr reset_init    
    lda #XEX_LOAD
    jsr emu_hook
    bmi error
    bcs done
    jsr call_init
    jmp loop
done:
    jsr call_run
finish:
    jmp *
error:
    lda #$30
    sta COLBK
    jmp *

call_run:
    cld
    jmp (RUNAD)

call_init:
    cld
    jmp (INITAD)

reset_init:
    pha
    lda #<do_nothing
    sta INITAD
    lda #>do_nothing
    sta INITAD+1
    pla

do_nothing:
    rts

.rept $780-3-*
    dta 0
.endr

xex_curr_pos:
    :3 dta 0
