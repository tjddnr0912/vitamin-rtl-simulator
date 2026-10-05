#!/usr/bin/env python3
"""r3.py [--post=PATH] [--out=DIR] [name-filter...]: every cell — grounding/planner (g/out), r1/sound, r1/diff — on
PRE (frozen), PRE-H (when the earlier run had one) and POST2, 3 backends, plus staged POST2 and PRE. Oracle
sections (verilator / iverilog / sv2v) are copied verbatim from the earlier output when it has them; otherwise
verilator is run here (same command as g/r.py). Multi-source cells use r1/diff's sidecars (.files/.srcs/.vargs).
Writes <out>/<group>__<base>.out (default g/out3), cell copies in g/w3/<group>__<base>."""
import os, re, sys, shutil, glob, subprocess
G = os.path.dirname(os.path.abspath(__file__))
S5 = os.path.dirname(G)
R1 = os.path.join(S5, "r1")
PRE = os.path.join(S5, "pre", "vita")
ENV = dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")


def sh(cmd, cwd, env=ENV, timeout=180):
    try:
        p = subprocess.run(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout)
        return p.returncode, p.stdout.decode("utf-8", "replace")
    except subprocess.TimeoutExpired as e:
        return "TIMEOUT", (e.stdout or b"").decode("utf-8", "replace")


def clean(o, w):
    o = o.replace(w + "/", "")
    return "\n".join(l for l in o.splitlines() if "VITA-W1017" not in l and not l.startswith("- S i m u")
                     and not l.startswith("- Verilator: ") and "ignored due to +verilator" not in l)


def vita3(tag, b, srcs, vargs, w, res, out):
    for be in ("native", "interp", "vm"):
        rc, o = sh([b, "--backend", be, "-o", f"{tag}_{be}.vcd"] + vargs + srcs, w)
        res[(tag, be)] = f"rc={rc}\n{clean(o, w)}"
    if res[(tag, "native")] == res[(tag, "interp")] == res[(tag, "vm")]:
        out.append(f"=== vita {tag} (native=interp=vm)\n{res[(tag, 'native')]}")
    else:
        for be in ("native", "interp", "vm"):
            out.append(f"=== vita {tag} {be} [BACKEND SPLIT]\n{res[(tag, be)]}")


def staged(tag, b, srcs, vargs, w, res, out):
    d = os.path.join(w, "st_" + tag); os.makedirs(d, exist_ok=True)
    for f in os.listdir(w):
        p = os.path.join(w, f)
        if os.path.isfile(p) and f.endswith((".sv", ".svh", ".v", ".vh", ".f")):
            shutil.copy(p, d)
    base = os.path.splitext(os.path.basename(srcs[0]))[0]
    r1 = sh([b, "vcmp"] + vargs + srcs, d); r2 = sh([b, "velab", base + ".vu"], d); r3 = sh([b, "vrun", "-o", "st.vcd", base + ".velab"], d)
    st = f"rc={r3[0]}\n{clean(r3[1], d)}"
    keep = lambda t: [l for l in t.splitlines() if "W4031" in l or "E4003" in l or not (re.search(r"(warning|note|info)\[VITA-", l) or l.startswith(("errors=", "rc=")))]
    same = keep(res[(tag, "native")]) == keep(st)
    out.append(f"--- {tag} staged vcmp={r1[0]} velab={r2[0]} vrun={r3[0]} W4031/E4003+stdout==one-shot: {same}"
               + ("" if same else f"\n{clean(r1[1], d)}\n{clean(r2[1], d)}\n{st}"))


def sections(txt):
    return ["=== " + s.split("\n--- ")[0].rstrip("\n") for s in re.split(r"^=== ", txt, flags=re.M)[1:]]


def side(c, ext):
    p = os.path.splitext(c)[0] + ext
    return open(p).read().split() if os.path.exists(p) else None


def specs():
    """(group, base, main source path, extra files [(src, rel)], srcs, vargs, vlargs, H path or None, old out or None)"""
    out = []
    for o in sorted(glob.glob(os.path.join(G, "out", "*.out"))):
        base = os.path.basename(o)[:-4]; w0 = os.path.join(G, "w", base); old = open(o).read()
        hs = [f for f in os.listdir(w0) if f.endswith(".sv") and f != base + ".sv" and os.path.isfile(os.path.join(w0, f))]
        h = os.path.join(w0, hs[0]) if "=== vita PRE-H" in old and len(hs) == 1 else None
        out.append(("g", base, os.path.join(w0, base + ".sv"), [], [base + ".sv"], [], [], h, o))
    sc = os.path.join(R1, "sound", "cells")
    seen = set()
    for c in sorted(glob.glob(os.path.join(sc, "*.sv"))):
        base = os.path.basename(c)[:-3]
        if base.endswith("_H"):
            continue
        o = os.path.join(R1, "sound", "out", base + ".out")
        o = o if os.path.exists(o) else None
        hp = os.path.join(sc, base + "_H.sv")
        h = hp if os.path.exists(hp) and (o is None or "=== vita PRE-H" in open(o).read()) else None
        out.append(("s", base, c, [], [base + ".sv"], [], [], h, o)); seen.add(base)
    out.append(("s", "q1d_include", os.path.join(sc, "q1d_inc", "q1d_include.sv"), [(os.path.join(sc, "q1d_inc", "chain.svh"), "chain.svh")], ["q1d_include.sv"], [], [], None, None))
    out.append(("s", "q1e_multi", os.path.join(sc, "q1e_multi", "a.sv"), [(os.path.join(sc, "q1e_multi", "b.sv"), "b.sv")], ["a.sv", "b.sv"], [], [], None, None))
    for b in ("q3vu_fn_only", "q3vu_pos"):
        out.append(("s", b, os.path.join(sc, "q3vu", b + ".sv"), [], [b + ".sv"], [], [], None, None))
    dc = os.path.join(R1, "diff", "cells")
    for o in sorted(glob.glob(os.path.join(R1, "diff", "out", "*.out"))):
        base = os.path.basename(o)[:-4]
        c = next((p for p in (os.path.join(dc, base + ".sv"), os.path.join(dc, "eq", base + ".sv"),
                              os.path.join(R1, "diff", "w", base, base + ".sv")) if os.path.exists(p)), None)
        cdir = os.path.dirname(c)
        files = [(os.path.join(cdir, f), f) for f in (side(c, ".files") or [])]
        out.append(("d", base, c, files, side(c, ".srcs") or [base + ".sv"], side(c, ".vargs") or [], side(c, ".vlargs") or [], None, o))
    return out


def main():
    args = sys.argv[1:]
    opts = {a.split("=")[0]: a.split("=", 1)[1] for a in args if a.startswith("--") and "=" in a}
    filt = [a for a in args if not a.startswith("--")]
    POST = opts.get("--post", os.path.join(S5, "post2", "vita"))
    OUT = opts.get("--out", os.path.join(G, "out3")); W = os.path.join(G, "w3")
    os.makedirs(OUT, exist_ok=True)
    for grp, base, c, files, srcs, vargs, vlargs, h, o in specs():
        name = f"{grp}__{base}"
        if filt and not any(f in name for f in filt):
            continue
        w = os.path.join(W, name); shutil.rmtree(w, ignore_errors=True); os.makedirs(w)
        shutil.copy(c, os.path.join(w, os.path.basename(c) if grp == "s" and base in ("q1d_include", "q1e_multi") else base + ".sv"))
        for src, rel in files:
            dst = os.path.join(w, rel); os.makedirs(os.path.dirname(dst), exist_ok=True); shutil.copy(src, dst)
        out = [f"##### {name}"]
        old = open(o).read() if o else ""
        orc = [s for s in sections(old) if s.startswith(("=== verilator", "=== iverilog", "=== sv2v"))]
        if any(s.startswith("=== verilator") for s in orc):
            out += orc
        else:
            rc, ot = sh(["verilator", "--binary", "--timing", "--assert", "-Wno-fatal", "-Wno-lint", "-Wno-style",
                         "--Mdir", "obj", "--top-module", "top"] + vlargs + srcs, w, timeout=400)
            if rc == 0:
                warn = "\n".join(l for l in ot.splitlines() if l.startswith("%Warning") or l.startswith("%Error"))
                rc2, o2 = sh([os.path.join(w, "obj", "Vtop"), "+verilator+error+limit+1000"], w)
                out.append(f"=== verilator compile rc=0{(' msgs: ' + warn) if warn else ''} run rc={rc2} [run by r3]\n{clean(o2, w)}")
            else:
                out.append(f"=== verilator compile rc={rc} [run by r3]\n" + "\n".join(clean(ot, w).splitlines()[:12]))
            out += [s for s in orc if not s.startswith("=== verilator")]
        res = {}
        vita3("PRE", PRE, srcs, vargs, w, res, out)
        if h:
            shutil.copy(h, w); hb = os.path.basename(h)
            vita3("PRE-H", PRE, [hb] + srcs[1:], vargs, w, res, out)
        vita3("POST", POST, srcs, vargs, w, res, out)
        same = all(res[("PRE", be)] == res[("POST", be)] for be in ("native", "interp", "vm"))
        out.append(f"--- PRE==POST (all backends): {same}")
        staged("POST", POST, srcs, vargs, w, res, out)
        staged("PRE", PRE, srcs, vargs, w, res, out)
        open(os.path.join(OUT, name + ".out"), "w").write("\n".join(out) + "\n")
        print(name, "done", flush=True)


main()
