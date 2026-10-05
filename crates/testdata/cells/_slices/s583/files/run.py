#!/usr/bin/env python3
"""Run one or more SV cells on iverilog, verilator(--assert), sv2v->iverilog, vita x3 backends.
Usage: run.py cell1.sv [cell2.sv ...]   -> writes <cell>.out next to each cell and prints it.
"""
import os, subprocess, sys, time, shutil

S = os.path.dirname(os.path.abspath(__file__))
VITA = os.path.join(S, "pre", "vita")
SV2V = os.path.join(os.path.dirname(S), "s580", "sv2v", "sv2v-macOS", "sv2v")
ENV = dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")


def sh(cmd, cwd, timeout=120):
    t = time.time()
    try:
        p = subprocess.run(cmd, cwd=cwd, env=ENV, stdout=subprocess.PIPE, stderr=subprocess.STDOUT,
                           timeout=timeout)
        return p.returncode, p.stdout.decode("utf-8", "replace"), time.time() - t
    except subprocess.TimeoutExpired as e:
        return "TIMEOUT", (e.stdout or b"").decode("utf-8", "replace"), time.time() - t


def run_cell(path, tools):
    path = os.path.abspath(path)
    d = os.path.dirname(path)
    base = os.path.splitext(os.path.basename(path))[0]
    w = os.path.join(d, "w_" + base)
    os.makedirs(w, exist_ok=True)
    out = []
    if "iv" in tools:
        rc, o, _ = sh(["iverilog", "-g2012", "-o", os.path.join(w, "a.vvp"), path], w)
        if rc == 0:
            rc2, o2, _ = sh(["vvp", "-n", os.path.join(w, "a.vvp")], w)
            out.append(("iverilog", f"compile rc=0{(' msgs: ' + o.strip()) if o.strip() else ''}\nrun rc={rc2}\n{o2}"))
        else:
            out.append(("iverilog", f"compile rc={rc}\n{o}"))
    if "vl" in tools:
        md = os.path.join(w, "obj_vl")
        if os.path.exists(md):
            shutil.rmtree(md)
        rc, o, _ = sh(["verilator", "--binary", "--timing", "--assert", "-Wno-fatal", "-Wno-lint",
                       "-Wno-style", "--Mdir", md, "--top-module", "top", path], w, timeout=300)
        if rc == 0:
            rc2, o2, _ = sh([os.path.join(md, "Vtop"), "+verilator+error+limit+1000"], w)
            warn = "\n".join(l for l in o.splitlines() if l.startswith("%Warning") or l.startswith("%Error"))
            out.append(("verilator", f"compile rc=0{(' msgs: ' + warn) if warn else ''}\nrun rc={rc2}\n{o2}"))
        else:
            out.append(("verilator", f"compile rc={rc}\n" + "\n".join(o.splitlines()[:15])))
    if "s2" in tools:
        v = os.path.join(w, "sv2v.v")
        rc, o, _ = sh([SV2V, path], w)
        if rc == 0:
            open(v, "w").write(o)
            rc1, o1, _ = sh(["iverilog", "-g2012", "-o", os.path.join(w, "b.vvp"), v], w)
            if rc1 == 0:
                rc2, o2, _ = sh(["vvp", "-n", os.path.join(w, "b.vvp")], w)
                out.append(("sv2v->iv", f"run rc={rc2}\n{o2}"))
            else:
                out.append(("sv2v->iv", f"iverilog compile rc={rc1}\n{o1}"))
        else:
            out.append(("sv2v->iv", f"sv2v rc={rc}\n{o}"))
    for be in ("native", "interp", "vm"):
        if "vita" not in tools and be not in tools:
            continue
        rc, o, _ = sh([VITA, "--backend", be, "-o", os.path.join(w, f"v_{be}.vcd"), path], w)
        out.append((f"vita-{be}", f"rc={rc}\n{o}"))
    # collapse identical vita backends
    vk = [i for i, (k, _) in enumerate(out) if k.startswith("vita-")]
    if len(vk) == 3 and out[vk[0]][1] == out[vk[1]][1] == out[vk[2]][1]:
        out[vk[0]] = ("vita (native=interp=vm)", out[vk[0]][1])
        out = [o for i, o in enumerate(out) if i not in vk[1:]]
    txt = f"##### {path}\n" + "".join(f"=== {k}\n{v.rstrip()}\n" for k, v in out)
    txt = txt.replace(d + "/w_" + base + "/", "").replace(d + "/", "")
    txt = "\n".join(l for l in txt.splitlines() if not l.startswith("warning[VITA-W1017]") and not l.startswith("- S i m u") and not l.startswith("- Verilator: ") and "ignored due to +verilator+error+limit" not in l) + "\n"
    # collapse identical vita backends
    open(os.path.join(d, base + ".out"), "w").write(txt)
    return txt


if __name__ == "__main__":
    args = sys.argv[1:]
    tools = {"iv", "vl", "vita"}
    if args and args[0].startswith("--tools="):
        tools = set(args[0][8:].split(","))
        args = args[1:]
    for p in args:
        print(run_cell(p, tools))
