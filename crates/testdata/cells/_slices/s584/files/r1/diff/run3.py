#!/usr/bin/env python3
"""run3.py cell.sv ... : iv, vl, vita {pre,a,ab}x{native,interp,vm}; writes <cell>.out; parallel over cells."""
import os, subprocess, sys, time, shutil, concurrent.futures as cf
D = os.path.dirname(os.path.abspath(__file__))
R1 = os.path.dirname(D)
BINS = [("pre", os.path.join(R1, "vita_pre")), ("a", os.path.join(R1, "vita_a")), ("ab", os.path.join(R1, "vita_ab"))]
ENV = dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")
ENV.pop("VITA_T0RULE", None)
DROP = ("warning[VITA-W1017]", "- S i m u", "- Verilator: ")
def sh(cmd, cwd, timeout=120):
    try:
        p = subprocess.run(cmd, cwd=cwd, env=ENV, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout)
        return p.returncode, p.stdout.decode("utf-8", "replace")
    except subprocess.TimeoutExpired as e:
        return "TIMEOUT", (e.stdout or b"").decode("utf-8", "replace")
def clean(t, d, w):
    t = t.replace(w + "/", "").replace(d + "/", "")
    return "\n".join(l for l in t.splitlines() if not l.startswith(DROP) and "ignored due to +verilator+error+limit" not in l).rstrip()
def run_cell(path, tools):
    path = os.path.abspath(path); d = os.path.dirname(path)
    base = os.path.splitext(os.path.basename(path))[0]
    w = os.path.join(d, "w_" + base); os.makedirs(w, exist_ok=True)
    out = []
    if "iv" in tools:
        rc, o = sh(["iverilog", "-g2012", "-o", "a.vvp", path], w)
        if rc == 0:
            rc2, o2 = sh(["vvp", "-n", "a.vvp"], w)
            out.append(("iverilog", f"compile rc=0{(' msgs: ' + o.strip()) if o.strip() else ''}\nrun rc={rc2}\n{o2}"))
        else:
            out.append(("iverilog", f"compile rc={rc}\n{o}"))
    if "vl" in tools:
        md = os.path.join(w, "obj_vl")
        if os.path.exists(md): shutil.rmtree(md)
        rc, o = sh(["verilator", "--binary", "--timing", "--assert", "-Wno-fatal", "-Wno-lint", "-Wno-style", "--Mdir", md, "--top-module", "top", path], w, timeout=400)
        if rc == 0:
            rc2, o2 = sh([os.path.join(md, "Vtop"), "+verilator+error+limit+1000"], w)
            warn = "\n".join(l for l in o.splitlines() if l.startswith("%Warning") or l.startswith("%Error"))
            out.append(("verilator", f"compile rc=0{(' msgs: ' + warn) if warn else ''}\nrun rc={rc2}\n{o2}"))
        else:
            out.append(("verilator", f"compile rc={rc}\n" + "\n".join(o.splitlines()[:12])))
    vres = []
    for lab, b in BINS:
        for be in ("native", "interp", "vm"):
            rc, o = sh([b, "--backend", be, "-o", f"v_{lab}_{be}.vcd", path], w)
            vres.append((f"{lab}-{be}", f"rc={rc}\n{o}"))
    groups = []
    for k, v in vres:
        cv = clean(v, d, w)
        for g in groups:
            if g[1] == cv: g[0].append(k); break
        else: groups.append([[k], cv])
    txt = f"##### {base}\n" + "".join(f"=== {k}\n{clean(v, d, w)}\n" for k, v in out)
    for ks, v in groups:
        txt += f"=== vita[{' '.join(ks)}]\n{v}\n"
    open(os.path.join(d, base + ".out"), "w").write(txt)
    return txt
if __name__ == "__main__":
    args = sys.argv[1:]; tools = {"iv", "vl"}
    if args and args[0].startswith("--tools="):
        tools = set(args[0][8:].split(",")); args = args[1:]
    with cf.ThreadPoolExecutor(4) as ex:
        for t in ex.map(lambda p: run_cell(p, tools), args):
            print(t, flush=True)
