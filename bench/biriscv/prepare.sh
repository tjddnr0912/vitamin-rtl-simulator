#!/bin/sh
# Regenerate prog.hex from the pinned upstream ELF.
#
# The image is NOT committed: it is extracted from upstream's own test.elf
# (Apache-2.0), so it is third-party content, and this repository redistributes none.
# `corpus-runner fetch --run` runs this after the clone.
set -e
cd "$(dirname "$0")"
python3 mkhex.py src/tb/tb_core_icarus/test.elf > prog.hex
wc -l < prog.hex | tr -d ' ' | sed 's/^/prog.hex lines: /'
