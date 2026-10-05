#!/usr/bin/env python3
"""r2.py [base...]: re-run every cell of S/s585/g/out with r.py's vita3/staged on PRE (frozen) and POST-c2
(S/s585/post/vita); oracle sections (verilator / iverilog / sv2v) are copied verbatim from the r.py output in
g/out (the oracles did not change). Writes g/out2/<base>.out (cell copies in g/w2/<base>)."""
import os, re, shutil, sys
G = os.path.dirname(os.path.abspath(__file__))
src = open(os.path.join(G, "r.py")).read().rsplit("\nmain()", 1)[0]
ns = {"__file__": os.path.join(G, "r.py"), "__name__": "r"}
exec(src, ns)
S = os.path.dirname(os.path.dirname(G))
PRE = os.path.join(S, "s585", "pre", "vita")
POST = os.path.join(S, "s585", "post", "vita")
OLD, OUT2, W2 = os.path.join(G, "out"), os.path.join(G, "out2"), os.path.join(G, "w2")
vita3, staged = ns["vita3"], ns["staged"]


def sections(txt):
    return ["=== " + s for s in re.split(r"^=== ", txt, flags=re.M)[1:]]


def main():
    bases = sys.argv[1:] or sorted(f[:-4] for f in os.listdir(OLD) if f.endswith(".out"))
    os.makedirs(OUT2, exist_ok=True)
    for base in bases:
        old = open(os.path.join(OLD, base + ".out")).read()
        w0 = os.path.join(G, "w", base)
        w = os.path.join(W2, base); shutil.rmtree(w, ignore_errors=True); os.makedirs(w)
        srcf = base + ".sv"; shutil.copy(os.path.join(w0, srcf), w)
        hs = [f for f in os.listdir(w0) if f.endswith(".sv") and f != srcf and os.path.isfile(os.path.join(w0, f))]
        has_h = "=== vita PRE-H" in old
        if has_h and len(hs) != 1:
            print(f"!! {base}: PRE-H section but H candidates {hs}"); continue
        out = [f"##### {base}"]
        for s in sections(old):
            if s.startswith(("=== verilator", "=== iverilog", "=== sv2v")):
                out.append(s.rstrip("\n").split("\n--- ")[0])
        res = {}
        vita3("PRE", PRE, srcf, w, res, out)
        if has_h:
            shutil.copy(os.path.join(w0, hs[0]), w)
            vita3("PRE-H", PRE, hs[0], w, res, out)
        vita3("POST", POST, srcf, w, res, out)
        same = all(res[("PRE", be)] == res[("POST", be)] for be in ("native", "interp", "vm"))
        out.append(f"--- PRE==POST (all backends): {same}")
        staged("POST", POST, srcf, base, w, res, out)
        staged("PRE", PRE, srcf, base, w, res, out)
        txt = "\n".join(out) + "\n"
        open(os.path.join(OUT2, base + ".out"), "w").write(txt)
        print(f"{base} done")


main()
