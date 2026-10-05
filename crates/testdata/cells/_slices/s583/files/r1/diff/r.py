#!/usr/bin/env python3
"""r.py [--novl] [--staged] [--jit] [--preh=H.sv] cell.sv -> verilator, PRE x3, POST x3 (+staged, +jit). Writes <cell>.out."""
import os, subprocess, sys, shutil
D = os.path.dirname(os.path.abspath(__file__)); S = os.path.dirname(os.path.dirname(D))
PRE, POST = os.path.join(S, "pre", "vita"), os.path.join(S, "post", "vita")
PREJ, POSTJ = os.path.join(S, "pre-jit", "vita"), os.path.join(S, "post-jit", "vita")
ENV = dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")
def sh(cmd, cwd, env=ENV, timeout=120):
    try:
        p = subprocess.run(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout)
        return p.returncode, p.stdout.decode("utf-8", "replace")
    except subprocess.TimeoutExpired as e:
        return "TIMEOUT", (e.stdout or b"").decode("utf-8", "replace")
def clean(o, w):
    o = o.replace(w + "/", "")
    return "\n".join(l for l in o.splitlines() if "VITA-W1017" not in l and not l.startswith("- S i m u") and not l.startswith("- Verilator: ") and "ignored due to +verilator" not in l)
args = sys.argv[1:]; opts = {a.split("=")[0]: (a.split("=")[1] if "=" in a else True) for a in args if a.startswith("--")}
cells = [a for a in args if not a.startswith("--")]
for c in cells:
    c = os.path.abspath(c); base = os.path.splitext(os.path.basename(c))[0]
    w = os.path.join(D, "w", base); shutil.rmtree(w, ignore_errors=True); os.makedirs(w)
    shutil.copy(c, os.path.join(w, base + ".sv")); src = base + ".sv"
    out = [f"##### {base}"]
    if "--novl" not in opts:
        rc, o = sh(["verilator", "--binary", "--timing", "--assert", "-Wno-fatal", "-Wno-lint", "-Wno-style", "--Mdir", "obj", "--top-module", "top", src], w, timeout=300)
        if rc == 0:
            warn = "\n".join(l for l in o.splitlines() if l.startswith("%Warning") or l.startswith("%Error"))
            rc2, o2 = sh([os.path.join(w, "obj", "Vtop"), "+verilator+error+limit+1000"], w)
            out.append(f"=== verilator compile rc=0{(' msgs: ' + warn) if warn else ''} run rc={rc2}\n{clean(o2, w)}")
        else:
            out.append(f"=== verilator compile rc={rc}\n" + "\n".join(o.splitlines()[:12]))
    res = {}
    runs = [("PRE", PRE, src), ("POST", POST, src)]
    if "--preh" in opts:
        h = os.path.abspath(opts["--preh"]); shutil.copy(h, os.path.join(w, os.path.basename(h))); runs.insert(1, ("PRE-H", PRE, os.path.basename(h)))
    for tag, b, f in runs:
        for be in ("native", "interp", "vm"):
            rc, o = sh([b, "--backend", be, "-o", f"{tag}_{be}.vcd", f], w)
            res[(tag, be)] = f"rc={rc}\n{clean(o, w)}"
        if res[(tag, "native")] == res[(tag, "interp")] == res[(tag, "vm")]:
            out.append(f"=== vita {tag} (native=interp=vm)\n{res[(tag, 'native')]}")
        else:
            for be in ("native", "interp", "vm"): out.append(f"=== vita {tag} {be} [BACKEND SPLIT]\n{res[(tag, be)]}")
    out.append(f"--- PRE==POST (all backends): {all(res[('PRE', be)] == res[('POST', be)] for be in ('native','interp','vm'))}")
    if "--staged" in opts:
        r1 = sh([POST, "vcmp", src], w); r2 = sh([POST, "velab", base + ".vu"], w); r3 = sh([POST, "vrun", "-o", "st.vcd", base + ".velab"], w)
        st = f"rc={r3[0]}\n{clean(r3[1], w)}"
        one_w = [l for l in res[("POST", "native")].splitlines() if "W4031" in l or not l.startswith(("warning", "note", "info", "errors="))]
        st_w = [l for l in st.splitlines() if "W4031" in l or not l.startswith(("warning", "note", "info", "errors="))]
        out.append(f"--- staged vcmp={r1[0]} velab={r2[0]} vrun={r3[0]} W4031+stdout==one-shot: {one_w == st_w}" + ("" if one_w == st_w else f"\n{st}"))
    if "--jit" in opts:
        je = dict(ENV, VITA_JIT="1", VITA_JIT_STATS="1")
        for tag, b, f in (("PRE-jit", PREJ, opts.get("--jith") and os.path.basename(opts["--jith"]) or src), ("POST-jit", POSTJ, src)):
            if opts.get("--jith"): shutil.copy(os.path.abspath(opts["--jith"]), w)
            rc, o = sh([b, "-o", f"{tag}.vcd", f], w, env=je)
            out.append(f"=== {tag} (VITA_JIT=1, file {f})\nrc={rc}\n{clean(o, w)}")
    txt = "\n".join(out) + "\n"
    open(os.path.join(os.path.dirname(c), base + ".out"), "w").write(txt); print(txt)
