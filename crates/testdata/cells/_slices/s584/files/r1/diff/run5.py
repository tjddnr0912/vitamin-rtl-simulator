#!/usr/bin/env python3
"""run5.py [--oracles] cells: vita {pre,post2,post3}; oracles from <cell>.out2 (or fresh with --oracles). writes .out3"""
import os, sys, concurrent.futures as cf
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import run3
D = os.path.dirname(os.path.abspath(__file__)); R1 = os.path.dirname(D); S = os.path.dirname(R1)
run3.BINS = [("pre", os.path.join(R1, "vita_pre")), ("post2", os.path.join(S, "r2", "vita_post2")), ("post3", os.path.join(S, "r3", "vita_post3"))]
def one(path, orc):
    path = os.path.abspath(path); d = os.path.dirname(path); base = os.path.splitext(os.path.basename(path))[0]
    src = os.path.join(d, base + ".out2"); r1o = os.path.join(d, base + ".out")
    keep = open(src).read() if (not orc and os.path.exists(src)) else None
    save = open(r1o).read() if os.path.exists(r1o) else None
    txt = run3.run_cell(path, set() if keep is not None else {"iv", "vl"})
    if keep is not None: txt = keep.split("=== vita[")[0] + "=== vita[" + txt.split("=== vita[", 1)[1]
    if save is not None: open(r1o, "w").write(save)
    open(os.path.join(d, base + ".out3"), "w").write(txt); return txt
if __name__ == "__main__":
    a = sys.argv[1:]; orc = a[:1] == ["--oracles"]; a = a[1:] if orc else a
    with cf.ThreadPoolExecutor(4) as ex: list(ex.map(lambda p: one(p, orc), a))
