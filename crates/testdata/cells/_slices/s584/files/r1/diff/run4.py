#!/usr/bin/env python3
"""run4.py [--oracles] cell.sv ... : vita {pre,a,post2}x{native,interp,vm}; iv/vl copied from r1 <cell>.out unless --oracles.
writes <cell>.out2"""
import os, sys, concurrent.futures as cf
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import run3
D = os.path.dirname(os.path.abspath(__file__)); R1 = os.path.dirname(D); S = os.path.dirname(R1)
run3.BINS = [("pre", os.path.join(R1, "vita_pre")), ("a", os.path.join(R1, "vita_a")), ("post2", os.path.join(S, "r2", "vita_post2"))]
def one(path, oracles):
    path = os.path.abspath(path); d = os.path.dirname(path); base = os.path.splitext(os.path.basename(path))[0]
    old = os.path.join(d, base + ".out")
    keep = open(old).read() if (not oracles and os.path.exists(old)) else None
    txt = run3.run_cell(path, {"iv", "vl"} if (oracles or keep is None) else set())
    if keep is not None:
        orc = keep.split("=== vita[")[0]
        txt = orc + "=== vita[" + txt.split("=== vita[", 1)[1]
    open(os.path.join(d, base + ".out2"), "w").write(txt)
    if keep is not None: open(old, "w").write(keep)   # run_cell overwrote .out; restore r1 text
    return txt
if __name__ == "__main__":
    a = sys.argv[1:]; orc = False
    if a and a[0] == "--oracles": orc = True; a = a[1:]
    with cf.ThreadPoolExecutor(4) as ex:
        for t in ex.map(lambda p: one(p, orc), a): print(t, flush=True)
