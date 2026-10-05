#!/usr/bin/env python3
"""h.py [--iv] [--s2v] [--novl] [--jit] [--staged] [--top=T] cell.sv...
r1 differential harness (adapted from S/s585/g/r.py; own dirs so g/out is untouched).
Sidecars next to a cell (optional): <cell>.files = extra files to copy (one per line, relative to the
cell's dir); <cell>.vargs = extra vita args placed before the source; <cell>.vlargs = extra verilator args;
<cell>.srcs = space-separated source list to use instead of the cell itself (vita + verilator + iverilog).
Writes r1/diff/out/<cell>.out and prints a summary + the full text."""
import os, re, subprocess, sys, shutil
D = os.path.dirname(os.path.abspath(__file__))
S = os.path.abspath(os.path.join(D, "..", "..", ".."))
PRE = os.path.join(S, "s585", "pre", "vita")
POST = os.path.join(S, "s585", "post", "vita")
JIT = os.path.join(S, "s585", "tgt-jit", "release", "vita")
SV2V = os.path.join(S, "s580", "sv2v", "sv2v-macOS", "sv2v")
ENV = dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")
OUT = os.path.join(D, "out"); W = os.path.join(D, "w")
VL_RE = re.compile(r"^\[(\d+)\] %Error: ([^:]+):(\d+): Assertion failed in ([^:]*): (.*)$", re.M)
VW_RE = re.compile(r"^(\S+?):(\d+):(\d+): (warning|error)\[(VITA-[WE]\d+)\][^\n]*?(?:\[at time (\d+)\])?$", re.M)


def sh(cmd, cwd, env=ENV, timeout=180):
    try:
        p = subprocess.run(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout)
        return p.returncode, p.stdout.decode("utf-8", "replace")
    except subprocess.TimeoutExpired as e:
        return "TIMEOUT", (e.stdout or b"").decode("utf-8", "replace")


def clean(o, w):
    o = o.replace(w + "/", "")
    return "\n".join(l for l in o.splitlines() if "VITA-W1017" not in l and not l.startswith("- S i m u")
                     and not l.startswith("- Verilator: ") and "ignored due to +verilator" not in l
                     and "no `timescale in the design" not in l)


def vsum(txt):
    r = []
    for m in VW_RE.finditer(txt):
        if m.group(5) in ("VITA-W4031", "VITA-E4003") or m.group(4) == "error":
            r.append(f"{m.group(5)[5:]}:{m.group(6) or 'elab'}@{m.group(1).split('/')[-1]}:{m.group(2)}:{m.group(3)}")
    return cap(r) or "-"


def cap(xs, n=40):
    return ",".join(xs[:n]) + (f",...(+{len(xs) - n})" if len(xs) > n else "")


def lsum(txt):
    return cap([f"{m.group(1)}@{m.group(2).split('/')[-1]}:{m.group(3)}" for m in VL_RE.finditer(txt)]) or "-"


def vita3(tag, b, srcs, vargs, w, res, out, env=ENV, bes=("native", "interp", "vm")):
    for be in bes:
        rc, o = sh([b, "--backend", be, "-o", f"{tag}_{be}.vcd"] + vargs + srcs, w, env=env)
        res[(tag, be)] = f"rc={rc}\n{clean(o, w)}"
    vals = [res[(tag, be)] for be in bes]
    if all(v == vals[0] for v in vals):
        out.append(f"=== vita {tag} ({'='.join(bes)})\n{vals[0]}")
    else:
        for be in bes:
            out.append(f"=== vita {tag} {be} [BACKEND SPLIT]\n{res[(tag, be)]}")


def staged(tag, b, srcs, w, res, out):
    d = os.path.join(w, "st_" + tag); os.makedirs(d, exist_ok=True)
    for f in os.listdir(w):
        p = os.path.join(w, f)
        if os.path.isfile(p) and (f.endswith((".sv", ".svh", ".v", ".vh", ".f"))):
            shutil.copy(p, d)
    base = os.path.splitext(os.path.basename(srcs[0]))[0]
    r1 = sh([b, "vcmp"] + srcs, d); r2 = sh([b, "velab", base + ".vu"], d); r3 = sh([b, "vrun", "-o", "st.vcd", base + ".velab"], d)
    st = f"rc={r3[0]}\n{clean(r3[1], d)}"
    keep = lambda t: [l for l in t.splitlines() if "W4031" in l or "E4003" in l or not (re.search(r"(warning|note|info)\[VITA-", l) or l.startswith(("errors=", "rc=")))]
    same = keep(res[(tag, "native")]) == keep(st)
    out.append(f"--- {tag} staged vcmp={r1[0]} velab={r2[0]} vrun={r3[0]} W4031/E4003+stdout==one-shot: {same}"
               + ("" if same else f"\n{clean(r1[1], d)}\n{clean(r2[1], d)}\n{st}"))


def side(c, ext):
    p = os.path.splitext(c)[0] + ext
    return open(p).read().split() if os.path.exists(p) else None


def main():
    args = sys.argv[1:]
    opts = {a.split("=")[0]: (a.split("=", 1)[1] if "=" in a else True) for a in args if a.startswith("--")}
    cells = [a for a in args if not a.startswith("--")]
    os.makedirs(OUT, exist_ok=True)
    for c in cells:
        c = os.path.abspath(c); base = os.path.splitext(os.path.basename(c))[0]; cdir = os.path.dirname(c)
        w = os.path.join(W, base); shutil.rmtree(w, ignore_errors=True); os.makedirs(w)
        shutil.copy(c, os.path.join(w, base + ".sv"))
        for f in side(c, ".files") or []:
            dst = os.path.join(w, f); os.makedirs(os.path.dirname(dst), exist_ok=True); shutil.copy(os.path.join(cdir, f), dst)
        srcs = side(c, ".srcs") or [base + ".sv"]
        vargs = side(c, ".vargs") or []
        vlargs = side(c, ".vlargs") or []
        top = opts.get("--top", "top")
        out = [f"##### {base}"]; summ = [base]
        if "--novl" not in opts:
            rc, o = sh(["verilator", "--binary", "--timing", "--assert", "-Wno-fatal", "-Wno-lint", "-Wno-style",
                        "--Mdir", "obj", "--top-module", top] + vlargs + srcs, w, timeout=400)
            if rc == 0:
                warn = "\n".join(l for l in o.splitlines() if l.startswith("%Warning") or l.startswith("%Error"))
                rc2, o2 = sh([os.path.join(w, "obj", "V" + top), "+verilator+error+limit+1000"], w)
                out.append(f"=== verilator compile rc=0{(' msgs: ' + warn) if warn else ''} run rc={rc2}\n{clean(o2, w)}")
                summ.append(f"vl:{lsum(o2)}")
            else:
                out.append(f"=== verilator compile rc={rc}\n" + "\n".join(clean(o, w).splitlines()[:12]))
                summ.append("vl:COMPILE-FAIL")
        if "--iv" in opts:
            rc, o = sh(["iverilog", "-g2012", "-o", "a.vvp"] + srcs, w)
            if rc == 0:
                rc2, o2 = sh(["vvp", "-n", "a.vvp"], w)
                out.append(f"=== iverilog compile rc=0{(' msgs: ' + o.strip()) if o.strip() else ''} run rc={rc2}\n{clean(o2, w)}")
                summ.append("iv:ran")
            else:
                out.append(f"=== iverilog compile rc={rc}\n" + "\n".join(clean(o, w).splitlines()[:8]))
                summ.append("iv:REJ")
        if "--s2v" in opts:
            rc, o = sh([SV2V] + srcs, w)
            if rc == 0:
                open(os.path.join(w, "s2v.v"), "w").write(o)
                rc1, o1 = sh(["iverilog", "-g2012", "-o", "b.vvp", "s2v.v"], w)
                if rc1 == 0:
                    rc2, o2 = sh(["vvp", "-n", "b.vvp"], w)
                    out.append(f"=== sv2v->iverilog run rc={rc2}\n{clean(o2, w)}")
                else:
                    out.append(f"=== sv2v->iverilog compile rc={rc1}\n" + "\n".join(o1.splitlines()[:6]))
            else:
                out.append(f"=== sv2v rc={rc}\n" + "\n".join(o.splitlines()[:6]))
        res = {}
        vita3("PRE", PRE, srcs, vargs, w, res, out)
        vita3("POST", POST, srcs, vargs, w, res, out)
        same = all(res[("PRE", be)] == res[("POST", be)] for be in ("native", "interp", "vm"))
        out.append(f"--- PRE==POST (all backends): {same}")
        pn, qn = res[("PRE", "native")], res[("POST", "native")]
        bsplit = any(res[(t, be)] != res[(t, "native")] for t in ("PRE", "POST") for be in ("interp", "vm"))
        summ.append(f"PRE:{vsum(pn)} {pn.splitlines()[0]}")
        summ.append(f"POST:{vsum(qn)} {qn.splitlines()[0]}")
        if "--jit" in opts:
            env = dict(ENV, VITA_JIT="1")
            vita3("POSTJIT", JIT, srcs, vargs, w, res, out, env=env, bes=("native",))
            summ.append(f"JIT==POST:{res[('POSTJIT','native')] == qn}")
        if "--staged" in opts:
            staged("POST", POST, srcs, w, res, out); staged("PRE", PRE, srcs, w, res, out)
        summ.append(f"P==P:{same}" + (" BACKEND-SPLIT" if bsplit else ""))
        txt = "\n".join(out) + "\n"
        open(os.path.join(OUT, base + ".out"), "w").write(" | ".join(summ) + "\n" + txt)
        print(" | ".join(summ))
        if "--quiet" not in opts:
            print(txt)


main()
