#!/usr/bin/env python3
"""r.py [--iv] [--s2v] [--novl] [--nopost] [--staged] [--preh=H.sv] cell.sv...
verilator (+iverilog, +sv2v->iverilog), PRE x3, [PRE-H x3], POST x3 (if built), [+staged POST and PRE].
Writes S/s585/g/out/<cell>.out and prints it."""
import os, re, subprocess, sys, shutil
G = os.path.dirname(os.path.abspath(__file__)); S = os.path.dirname(os.path.dirname(G))
PRE = os.path.join(S, "s585", "pre", "vita")
POST = os.path.join(S, "s585", "postc", "vita")
SV2V = os.path.join(S, "s580", "sv2v", "sv2v-macOS", "sv2v")
ENV = dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")
OUT = os.path.join(G, "out")


def sh(cmd, cwd, env=ENV, timeout=120):
    try:
        p = subprocess.run(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout)
        return p.returncode, p.stdout.decode("utf-8", "replace")
    except subprocess.TimeoutExpired as e:
        return "TIMEOUT", (e.stdout or b"").decode("utf-8", "replace")


def clean(o, w):
    o = o.replace(w + "/", "")
    return "\n".join(l for l in o.splitlines() if "VITA-W1017" not in l and not l.startswith("- S i m u")
                     and not l.startswith("- Verilator: ") and "ignored due to +verilator" not in l)


def vita3(tag, b, f, w, res, out):
    for be in ("native", "interp", "vm"):
        rc, o = sh([b, "--backend", be, "-o", f"{tag}_{be}.vcd", f], w)
        res[(tag, be)] = f"rc={rc}\n{clean(o, w)}"
    if res[(tag, "native")] == res[(tag, "interp")] == res[(tag, "vm")]:
        out.append(f"=== vita {tag} (native=interp=vm)\n{res[(tag, 'native')]}")
    else:
        for be in ("native", "interp", "vm"):
            out.append(f"=== vita {tag} {be} [BACKEND SPLIT]\n{res[(tag, be)]}")


def staged(tag, b, src, base, w, res, out):
    d = os.path.join(w, "st_" + tag); os.makedirs(d, exist_ok=True); shutil.copy(os.path.join(w, src), d)
    r1 = sh([b, "vcmp", src], d); r2 = sh([b, "velab", base + ".vu"], d); r3 = sh([b, "vrun", "-o", "st.vcd", base + ".velab"], d)
    st = f"rc={r3[0]}\n{clean(r3[1], d)}"
    keep = lambda t: [l for l in t.splitlines() if "W4031" in l or "E4003" in l or not (re.search(r"(warning|note|info)\[VITA-", l) or l.startswith(("errors=", "rc=")))]
    same = keep(res[(tag, "native")]) == keep(st)
    out.append(f"--- {tag} staged vcmp={r1[0]} velab={r2[0]} vrun={r3[0]} W4031/E4003+stdout==one-shot: {same}"
               + ("" if same else f"\n{clean(r1[1], d)}\n{clean(r2[1], d)}\n{st}"))


def main():
    args = sys.argv[1:]
    opts = {a.split("=")[0]: (a.split("=", 1)[1] if "=" in a else True) for a in args if a.startswith("--")}
    cells = [a for a in args if not a.startswith("--")]
    os.makedirs(OUT, exist_ok=True)
    for c in cells:
        c = os.path.abspath(c); base = os.path.splitext(os.path.basename(c))[0]
        w = os.path.join(G, "w", base); shutil.rmtree(w, ignore_errors=True); os.makedirs(w)
        shutil.copy(c, os.path.join(w, base + ".sv")); src = base + ".sv"
        out = [f"##### {base}"]
        if "--novl" not in opts:
            rc, o = sh(["verilator", "--binary", "--timing", "--assert", "-Wno-fatal", "-Wno-lint", "-Wno-style",
                        "--Mdir", "obj", "--top-module", opts.get("--top", "top"), src], w, timeout=300)
            if rc == 0:
                warn = "\n".join(l for l in o.splitlines() if l.startswith("%Warning") or l.startswith("%Error"))
                rc2, o2 = sh([os.path.join(w, "obj", "V" + opts.get("--top", "top")), "+verilator+error+limit+1000"], w)
                out.append(f"=== verilator compile rc=0{(' msgs: ' + warn) if warn else ''} run rc={rc2}\n{clean(o2, w)}")
            else:
                out.append(f"=== verilator compile rc={rc}\n" + "\n".join(clean(o, w).splitlines()[:12]))
        if "--iv" in opts:
            rc, o = sh(["iverilog", "-g2012", "-o", "a.vvp", src], w)
            if rc == 0:
                rc2, o2 = sh(["vvp", "-n", "a.vvp"], w)
                out.append(f"=== iverilog compile rc=0{(' msgs: ' + o.strip()) if o.strip() else ''} run rc={rc2}\n{clean(o2, w)}")
            else:
                out.append(f"=== iverilog compile rc={rc}\n" + "\n".join(clean(o, w).splitlines()[:8]))
        if "--s2v" in opts:
            rc, o = sh([SV2V, src], w)
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
        vita3("PRE", PRE, src, w, res, out)
        if "--preh" in opts:
            h = os.path.abspath(opts["--preh"]); shutil.copy(h, os.path.join(w, os.path.basename(h)))
            vita3("PRE-H", PRE, os.path.basename(h), w, res, out)
        if "--nopost" not in opts and os.path.exists(POST):
            vita3("POST", POST, src, w, res, out)
            same = all(res[("PRE", be)] == res[("POST", be)] for be in ("native", "interp", "vm"))
            out.append(f"--- PRE==POST (all backends): {same}")
            if "--staged" in opts:
                staged("POST", POST, src, base, w, res, out)
        if "--staged" in opts:
            staged("PRE", PRE, src, base, w, res, out)
        txt = "\n".join(out) + "\n"
        open(os.path.join(OUT, base + ".out"), "w").write(txt); print(txt)


main()
