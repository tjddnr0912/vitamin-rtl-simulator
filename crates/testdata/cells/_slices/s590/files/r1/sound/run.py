#!/usr/bin/env python3
import os, sys, subprocess, shutil, re
S = "/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad"
R = S + "/s590/r1/sound"
PRE = S + "/s590/pre2/vita"; POST = S + "/s590/post_a/vita"
ENV = dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")
def sh(cmd, cwd, t=60):
    try:
        p = subprocess.run(cmd, cwd=cwd, env=ENV, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=t)
        return p.returncode, p.stdout.decode("utf-8", "replace")
    except subprocess.TimeoutExpired as e:
        return "TIMEOUT", (e.stdout or b"").decode("utf-8", "replace")
def clean(o, w):
    return "\n".join(l.replace(w + "/", "") for l in o.splitlines() if "VITA-W1017" not in l and not l.startswith("- ")
                     and "ignored due to +verilator" not in l)
for c in sys.argv[1:]:
    base = os.path.splitext(os.path.basename(c))[0]
    w = R + "/w/" + base; shutil.rmtree(w, ignore_errors=True); os.makedirs(w); shutil.copy(c, w + "/" + base + ".sv"); src = base + ".sv"
    out = ["##### " + base]
    rc, o = sh(["iverilog", "-g2012", "-o", "a.vvp", src], w)
    if rc == 0:
        rc2, o2 = sh(["vvp", "-n", "a.vvp"], w); out.append(f"=== iverilog run rc={rc2}\n{clean(o2, w)}")
    else:
        out.append(f"=== iverilog compile rc={rc}\n{clean(o, w)[:600]}")
    rc, o = sh(["verilator", "--binary", "--timing", "--assert", "-Wno-fatal", "-Wno-lint", "-Wno-style", "--Mdir", "obj", "--top-module", "top", src], w, 300)
    if rc == 0:
        rc2, o2 = sh([w + "/obj/Vtop"], w); out.append(f"=== verilator run rc={rc2}\n{clean(o2, w)}")
        shutil.rmtree(w + "/obj", ignore_errors=True)
    else:
        out.append(f"=== verilator compile rc={rc}\n" + "\n".join(clean(o, w).splitlines()[:6]))
    res = {}
    for tag, b in (("PRE2", PRE), ("POST", POST)):
        for be in ("native", "interp", "vm"):
            rc, o = sh([b, "--backend", be, src], w)
            res[(tag, be)] = f"rc={rc}\n{clean(o, w)}"
        if res[(tag, "native")] == res[(tag, "interp")] == res[(tag, "vm")]:
            out.append(f"=== {tag} (native=interp=vm)\n{res[(tag, 'native')]}")
        else:
            for be in ("native", "interp", "vm"):
                out.append(f"=== {tag} {be} [BACKEND SPLIT]\n{res[(tag, be)]}")
    out.append("--- PRE2==POST per backend: " + " ".join(f"{be}={res[('PRE2', be)] == res[('POST', be)]}" for be in ("native", "interp", "vm")))
    t = "\n".join(out); open(R + "/out/" + base + ".out", "w").write(t + "\n")
