#!/usr/bin/env python3
"""Rerun every grounding + r1 lens cell on PRE, r1-A, POST2 x native/interp/vm. Out: r2run/<bin>/<id>.out"""
import os, subprocess, glob, concurrent.futures as cf
S = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
BINS = {"post3": os.path.join(S, "post3", "vita"), "post4": os.path.join(S, "post4", "vita")}
cells = []
for d in ["c1", "c1t", "c2", "c3/orig", "c4", "c6", "c7", "c8", "c9", "c10", "c11", "c12", "c13", "c14",
          "r1/diff/b1", "r1/diff/b2", "r1/diff/b3", "r1/diff/b4", "r1/diff/res", "r1/diff/r2t", "r1/diff/r2u", "r1/sound/cells"]:
    for f in sorted(glob.glob(os.path.join(S, d, "*.sv"))):
        if os.path.basename(f).startswith("p_comb_"):
            continue  # perf-size cells (lens N1), timed separately
        cells.append((d.replace("/", "_") + "__" + os.path.splitext(os.path.basename(f))[0], f))
env = dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")
for k in ("VITA_T0RULE", "VITA_JIT", "VITA_JIT_STATS"): env.pop(k, None)
def one(item):
    cid, f = item
    res = {}
    for b, v in BINS.items():
        outs = []
        for be in ("native", "interp", "vm"):
            try:
                p = subprocess.run([v, "--backend", be, f], cwd=os.path.dirname(f), env=env,
                                   stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=60)
                o = f"rc={p.returncode}\n" + p.stdout.decode("utf-8", "replace")
            except subprocess.TimeoutExpired as e:
                o = "rc=TIMEOUT\n"
            o = "\n".join(l for l in o.splitlines() if "VITA-W1017" not in l) + "\n"
            o = o.replace(os.path.dirname(f) + "/", "")
            outs.append(o)
        same = outs[0] == outs[1] == outs[2]
        txt = ("=== vita (native=interp=vm)\n" + outs[0]) if same else "".join(f"=== vita-{be}\n{o}" for be, o in zip(("native", "interp", "vm"), outs))
        os.makedirs(os.path.join(S, "r4run", b), exist_ok=True)
        open(os.path.join(S, "r4run", b, cid + ".out"), "w").write(txt)
        res[b] = (same, txt)
    return cid, f, res
with cf.ThreadPoolExecutor(8) as ex:
    out = list(ex.map(one, cells))
open(os.path.join(S, "r4run", "cells.tsv"), "w").write("\n".join(f"{c}\t{f}" for c, f, _ in out) + "\n")
nb = [c for c, _, r in out for b in r if not r[b][0]]
print("cells", len(out), "backend-mismatch", nb)
d_a = [c for c, _, r in out if r["post4"][1] != r["post3"][1]]
d_p = []
print("post4!=post3", len(d_a)); print("post3!=pre", len(d_p))
open(os.path.join(S, "r4run", "diff_vs_post2.txt"), "w").write("\n".join(d_a) + "\n")
