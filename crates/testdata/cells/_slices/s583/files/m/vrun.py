#!/usr/bin/env python3
"""vrun.py BIN cell.sv... -> vita x3 backends, collapsed when identical. Filters W1017."""
import os, subprocess, sys
ENV = dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")
binp = sys.argv[1]
for path in sys.argv[2:]:
    path = os.path.abspath(path)
    d = os.path.dirname(path); base = os.path.splitext(os.path.basename(path))[0]
    w = os.path.join(os.path.dirname(os.path.abspath(__file__)), "w_" + base); os.makedirs(w, exist_ok=True)
    outs = []
    for be in ("native", "interp", "vm"):
        p = subprocess.run([binp, "--backend", be, "-o", os.path.join(w, f"v_{be}.vcd"), path], cwd=d, env=ENV,
                           stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
        o = p.stdout.decode("utf-8", "replace")
        o = "\n".join(l for l in o.splitlines() if "VITA-W1017" not in l)
        outs.append((be, f"rc={p.returncode}\n{o}"))
    print(f"##### {os.path.basename(path)}")
    if outs[0][1] == outs[1][1] == outs[2][1]:
        print("=== vita (native=interp=vm)\n" + outs[0][1])
    else:
        for be, o in outs: print(f"=== vita-{be}\n{o}")
