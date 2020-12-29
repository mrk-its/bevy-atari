#!/usr/bin/env python3
import copy
import gzip
import pprint
import struct
import sys

Atari800_MACHINE_XLXE = 1

def read_int(f):
    return struct.unpack("i", f.read(4))[0]

def read_byte(f):
    return ord(f.read(1))

def read_word(f):
    return struct.unpack("H", f.read(2))[0]

def read_filename(f):
    return f.read(read_word(f))

def cartridge_state_read(f):
    saved_type = read_int(f)
    print(f"saved_type: {saved_type}")
    assert saved_type == 0, "reading cartridge state not supported"

def sio_state_read(f):
    sio = {}
    for i in range(8):
        sio[i] = {}
        sio[i]['status'] = read_int(f)
        sio[i]['filename'] = read_filename(f)
    return sio

def atari800_state_read(f):
    atari800 = {}

    atari800['is_pal'] = read_byte(f)
    atari800['machine_size'] = read_byte(f)

    if atari800['machine_size'] == Atari800_MACHINE_XLXE:
        atari800['builtin_basic'] = read_byte(f)
        atari800['keyboard_leds'] = read_byte(f)
        atari800['f_keys'] = read_byte(f)
        atari800['jumper'] = read_byte(f)
        atari800['builtin_game'] = read_byte(f)
        atari800['keyboard_detached'] = read_byte(f)
    return atari800

def antic_state_read(f):
    antic = {}
    antic['dmactl'] = read_byte(f)
    antic['chactl'] = read_byte(f)
    antic['hscrol'] = read_byte(f)
    antic['vscrol'] = read_byte(f)
    antic['pmbase'] = read_byte(f)
    antic['chbase'] = read_byte(f)
    antic['nmien'] = read_byte(f)
    antic['nmist'] = read_byte(f)
    antic['ir'] = read_byte(f)
    antic['anticmode'] = read_byte(f)
    antic['dctr'] = read_byte(f)
    antic['lastline'] = read_byte(f)
    antic['need_dl'] = read_byte(f)
    antic['vscrol_off'] = read_byte(f)
    antic['dlist'] = read_word(f)
    antic['screenaddr'] = read_word(f)
    antic['xpos'] = read_int(f)
    antic['xpos_limit'] = read_int(f)
    antic['ypos'] = read_int(f)
    return antic

def gtia_state_read(f):
    gtia = {}
    gtia['HPOSP0'] = read_byte(f)
    gtia['HPOSP1'] = read_byte(f)
    gtia['HPOSP2'] = read_byte(f)
    gtia['HPOSP3'] = read_byte(f)
    gtia['HPOSM0'] = read_byte(f)
    gtia['HPOSM1'] = read_byte(f)
    gtia['HPOSM2'] = read_byte(f)
    gtia['HPOSM3'] = read_byte(f)

    gtia['PF0PM'] = read_byte(f)
    gtia['PF1PM'] = read_byte(f)
    gtia['PF2PM'] = read_byte(f)
    gtia['PF3PM'] = read_byte(f)

    gtia['M0PL'] = read_byte(f)
    gtia['M1PL'] = read_byte(f)
    gtia['M2PL'] = read_byte(f)
    gtia['M3PL'] = read_byte(f)
    gtia['P0PL'] = read_byte(f)
    gtia['P1PL'] = read_byte(f)
    gtia['P2PL'] = read_byte(f)
    gtia['P3PL'] = read_byte(f)

    gtia['SIZEP0'] = read_byte(f)
    gtia['SIZEP1'] = read_byte(f)
    gtia['SIZEP2'] = read_byte(f)
    gtia['SIZEP3'] = read_byte(f)
    gtia['SIZEM'] = read_byte(f)

    gtia['GRAFP0'] = read_byte(f)
    gtia['GRAFP1'] = read_byte(f)
    gtia['GRAFP2'] = read_byte(f)
    gtia['GRAFP3'] = read_byte(f)
    gtia['GRAFM'] = read_byte(f)

    gtia['COLPM0'] = read_byte(f)
    gtia['COLPM1'] = read_byte(f)
    gtia['COLPM2'] = read_byte(f)
    gtia['COLPM3'] = read_byte(f)
    gtia['COLPF0'] = read_byte(f)
    gtia['COLPF1'] = read_byte(f)
    gtia['COLPF2'] = read_byte(f)
    gtia['COLPF3'] = read_byte(f)
    gtia['COLBK'] = read_byte(f)

    gtia['PRIOR'] = read_byte(f)
    gtia['VDELAY'] = read_byte(f)
    gtia['GRACTL'] = read_byte(f)
    gtia['CONSOL_MASK'] = read_byte(f)
    gtia['SPEAKER'] = read_int(f)
    read_int(f) # next_console_value? ignored
    gtia['TRIG_LATCH'] = read_int(f)
    return gtia

def memory_state_read(f, state):
    verbose = state['verbose']
    memory = {}
    memory['base_ram_kb'] = read_int(f)
    memory['data'] = f.read(65536)
    memory['attrib'] = f.read(65536)
    if state['atari800']['machine_size'] == Atari800_MACHINE_XLXE:
        if verbose:
            memory['basic'] = f.read(8192)
            print(repr(memory['basic']))
        memory['carta0bf'] = f.read(8192)
        if verbose:
            memory['os'] = f.read(16384)
        memory['under_atarixl_os'] = f.read(16384)
        if verbose:
            memory['xegame'] = f.read(0x2000)

    memory['num_xe_banks'] = read_int(f)

    ram_size = memory['base_ram_kb'] + memory['num_xe_banks'] * 16
    if ram_size == 320:
        xe_type = read_int(f)
        ram_size += xe_type
    assert ram_size in (64, 128)

    memory['portb'] = read_byte(f)
    memory['cart_a0bf_enabled'] = read_int(f)

    if ram_size > 64:
        atarixe_memory_size = (1 + (ram_size - 64) // 16) * 16384
        memory['atarixe_memory'] = f.read(atarixe_memory_size)

    if state['atari800']['machine_size'] == Atari800_MACHINE_XLXE and ram_size > 20:
        memory['enable_mapram'] = read_int(f)

    return memory

def cpu_state_read(f, state):
    cpu = {}
    cpu['reg_a'] = read_byte(f)
    cpu['reg_p'] = read_byte(f)
    cpu['reg_s'] = read_byte(f)
    cpu['reg_x'] = read_byte(f)
    cpu['reg_y'] = read_byte(f)
    cpu['irq'] = read_byte(f)
    cpu['memory'] = memory_state_read(f, state)
    cpu['pc'] = read_word(f)
    return cpu

def read_atari_state(f):
    state = {}
    magic = f.read(8)
    assert magic == b"ATARI800"

    state['version'] = read_byte(f)
    assert state['version'] >= 8

    state['verbose'] = read_byte(f)

    state['atari800'] = atari800_state_read(f)
    state['cartridge'] = cartridge_state_read(f)
    state['sio'] = sio_state_read(f)
    state['antic'] = antic_state_read(f)
    state['cpu'] = cpu_state_read(f, state)
    state['gtia'] = gtia_state_read(f)
    return state


def show_state(state):
    state_copy = copy.deepcopy(state)
    memory = state_copy['cpu']['memory']
    for k, v in list(memory.items()):
        if isinstance(v, bytes):
            memory[k] = f"[{len(v) // 1024} kb]"
    pprint.pprint(state_copy)

def memory_dump(data):
    for byte in data:
        print("{:02x} ".format(byte), end="")
    print()

if __name__ == "__main__":
    with gzip.open(sys.argv[1]) as f:
        state = read_atari_state(f)
        show_state(state)
        dlist = state['antic']['dlist']
        memory = state['cpu']['memory']['data']
        with open('memory.dat', 'wb') as fm:
            fm.write(memory)
        dlist_data = memory[dlist:dlist+256]

        memory_dump(dlist_data)
        print()
