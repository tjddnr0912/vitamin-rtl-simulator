#!/usr/bin/env python3
"""Run cells on the PROTOTYPE vita for each rule (VITA_T0RULE), interp + vm (+native when asked).
Usage: prun.py <outroot> <rules,comma> <backends,comma> cell.sv ...  -> <outroot>/<rule>/<cell>.out"""
import os, subprocess, sys, concurrent.futures as cf
S = os.path.dirname(os.path.abspath(__file__))
VITA = os.environ.get("PVITA", os.path.join(S, "tgt-proto", "debug", "vita"))
def run1(cell, rule, bes, outroot):
    base = os.path.splitext(os.path.basename(cell))[0]
    d = os.path.join(outroot, rule); os.makedirs(d, exist_ok=True)
    w = os.path.join(d, "w_" + base); os.makedirs(w, exist_ok=True)
    env = dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")
    if rule != "PRE": env["VITA_T0RULE"] = rule
    else: env.pop("VITA_T0RULE", None)
    res = []
    for be in bes:
        try:
            p = subprocess.run([VITA, "--backend", be, "-o", os.path.join(w, f"v_{be}.vcd"), cell], cwd=os.path.dirname(cell),
                               env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
            o = f"rc={p.returncode}\n" + p.stdout.decode("utf-8", "replace")
        except subprocess.TimeoutExpired as e:
            o = "rc=TIMEOUT\n" + (e.stdout or b"").decode("utf-8", "replace")
        o = "\n".join(l for l in o.splitlines() if not l.startswith("warning[VITA-W1017]")) + "\n"
        o = o.replace(os.path.dirname(cell) + "/", "")
        res.append((be, o))
    if all(r[1] == res[0][1] for r in res):
        txt = f"=== vita ({'='.join(b for b,_ in res)})\n" + res[0][1]
    else:
        txt = "".join(f"=== vita-{b}\n{o}" for b, o in res)
    open(os.path.join(d, base + ".out"), "w").write(txt)
if __name__ == "__main__":
    outroot, rules, bes = sys.argv[1], sys.argv[2].split(","), sys.argv[3].split(",")
    cells = [os.path.abspath(c) for c in sys.argv[4:]]
    with cf.ThreadPoolExecutor(8) as ex:
        futs = [ex.submit(run1, c, r, bes, outroot) for c in cells for r in rules]
        for f in futs: f.result()
    print("done", len(futs))
