# regenerate prog.hex from upstream src/tb/tb_core_icarus/test.elf
import struct, sys
elf = open(sys.argv[1], 'rb').read()
phoff, = struct.unpack_from('<I', elf, 0x1c)
phentsize, phnum = struct.unpack_from('<HH', elf, 0x2a)
img = {}
memsz_end = 0
for i in range(phnum):
    o = phoff + i*phentsize
    p_type, p_offset, p_vaddr, p_paddr, p_filesz, p_memsz = struct.unpack_from('<IIIIII', elf, o)
    if p_type != 1:  # PT_LOAD
        continue
    base = p_vaddr - 0x80000000
    for j in range(p_filesz):
        img[base + j] = elf[p_offset + j]
    memsz_end = max(memsz_end, base + p_memsz)
out = ''.join('%02x\n' % img.get(a, 0) for a in range(memsz_end))
sys.stdout.write(out)
sys.stderr.write('size=%d\n' % memsz_end)
