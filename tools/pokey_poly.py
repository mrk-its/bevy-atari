
def poly17():
    n_bits = 17
    highest_bit = 1 << (n_bits-1)
    v = highest_bit
    while True:
        yield v & 1
        v = (v >> 1) + (((v << (n_bits-1)) ^ (v << (12-1))) & highest_bit)
        if v == highest_bit:
            break

def poly9():
    n_bits = 9
    highest_bit = 1 << (n_bits-1)
    v = highest_bit
    while True:
        yield v & 1
        v = (v >> 1) + (((v << (n_bits-1)) ^ (v << (4-1))) & highest_bit)
        if v == highest_bit:
            break


def poly5():
    n_bits = 5
    highest_bit = 1 << (n_bits-1)
    v = highest_bit
    while True:
        yield v & 1
        v = (v >> 1) + (((v << (n_bits-1)) ^ (v << (3-1))) & highest_bit)
        if v == highest_bit:
            break

def poly4():
    n_bits = 4
    highest_bit = 1 << (n_bits-1)
    v = highest_bit
    while True:
        yield v & 1
        v = (v >> 1) + (((v << (n_bits-1)) ^ (v << (3-1))) & highest_bit)
        if v == highest_bit:
            break



print(len(list(poly4())), list(poly4()))
print(len(list(poly5())), list(poly5()))
print(len(list(poly9())), list(poly9()))
print(len(list(poly17())))

for n, fn in [(4, poly4), (5, poly5), (9, poly9), (17, poly17)]:
    with open(f"src/pokey/poly_{n}.dat", "wb") as f:
        f.write(bytes(fn()))
