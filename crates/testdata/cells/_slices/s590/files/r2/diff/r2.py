#!/usr/bin/env python3
"""r.py OUTDIR [--staged] cell.sv... : PRE2 x3, POST x3, iverilog (sv2v if refused), verilator, [staged PRE2/POST]."""
import os, re, subprocess, sys, shutil, difflib
S = "/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad"
PRE = f"{S}/s590/pre2/vita"; POSTA = f"{S}/s590/post_a/vita"; POSTC = f"{S}/s590/post_c/vita"
PRES = f"{S}/s590/pre2/sep"; POSTCS = f"{S}/s590/post_c/sep"
SV2V = f"{S}/tools/sv2v-macOS/sv2v"
ENV = dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")
WD = f"{S}/s590/r2/diff/w"
def sh(cmd, cwd, timeout=120):
    try:
        p = subprocess.run(cmd, cwd=cwd, env=ENV, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=timeout)
        return p.returncode, p.stdout.decode("utf-8", "replace")
    except subprocess.TimeoutExpired as e:
        return "TIMEOUT", (e.stdout or b"").decode("utf-8", "replace")
def clean(o, w):
    o = o.replace(w + "/", "")
    return "\n".join(l for l in o.splitlines() if "VITA-W1017" not in l and not l.startswith("- Verilator:") and "ignored due to +verilator" not in l and not l.startswith("VCD info:"))
def vcdnorm(p):
    try: t = open(p).read()
    except Exception: return None
    t = re.sub(r"\$date.*?\$end", "", t, flags=re.S); t = re.sub(r"\$version.*?\$end", "", t, flags=re.S)
    return t
def vita3(tag, b, src, w, out):
    res = {}; vcds = {}
    for be in ("native", "interp", "vm"):
        rc, o = sh([b, "--backend", be, "-o", f"{tag}_{be}.vcd", src], w)
        res[be] = f"rc={rc}\n{clean(o, w)}"; vcds[be] = vcdnorm(os.path.join(w, f"{tag}_{be}.vcd"))
    if res["native"] == res["interp"] == res["vm"]:
        out.append(f"=== {tag} (native=interp=vm)\n{res['native']}")
    else:
        for be in ("native", "interp", "vm"):
            out.append(f"=== {tag} {be} [BACKEND SPLIT]\n{res[be]}")
    return res, vcds
def staged(tag, d, src, base, w, out):
    sd = os.path.join(w, "st_" + tag); os.makedirs(sd, exist_ok=True); shutil.copy(os.path.join(w, src), sd)
    r1 = sh([f"{d}/vcmp", "-o", base + ".vu", src], sd)
    r2 = sh([f"{d}/velab", "-o", base + ".velab", base + ".vu"], sd)
    r3 = sh([f"{d}/vrun", "-o", "st.vcd", base + ".velab"], sd)
    t = f"rc={r3[0]}\n{clean(r3[1], sd)}"
    out.append(f"=== staged {tag} vcmp={r1[0]} velab={r2[0]}\n{t}")
    return t
def main():
    outdir = sys.argv[1]; args = sys.argv[2:]
    os.makedirs(outdir, exist_ok=True); summ = []
    for c in args:
        c = os.path.abspath(c); base = os.path.splitext(os.path.basename(c))[0]
        w = os.path.join(WD, base); shutil.rmtree(w, ignore_errors=True); os.makedirs(w)
        shutil.copy(c, os.path.join(w, base + ".sv")); src = base + ".sv"
        out = [f"##### {base}"]
        pre, pv = vita3("PRE2", PRE, src, w, out)
        pa, av = vita3("POSTA", POSTA, src, w, out)
        pc, cv = vita3("POSTC", POSTC, src, w, out)
        split = not (pc["native"] == pc["interp"] == pc["vm"])
        vac = all(av[b] == cv[b] for b in av); vpc = all(pv[b] == cv[b] for b in pv)
        out.append(f"--- PRE2==POSTA {pre == pa}  POSTA==POSTC {pa == pc}  PRE2==POSTC {pre == pc}  POSTC split {split}  VCD POSTA==POSTC {vac}  VCD PRE2==POSTC {vpc}")
        if not vac and av["native"] is not None and cv["native"] is not None:
            d = list(difflib.unified_diff(av["native"].splitlines(), cv["native"].splitlines(), "POSTA.vcd", "POSTC.vcd", n=1, lineterm=""))
            out.append("--- VCD diff POSTA->POSTC native (first 30):\n" + "\n".join(d[:30]))
        sp = staged("PRE2", PRES, src, base, w, out); sc = staged("POSTC", POSTCS, src, base, w, out)
        out.append(f"--- staged POSTC==native {sc == pc['native']}  staged PRE2==native {sp == pre['native']}")
        rc, o = sh(["iverilog", "-g2012", "-o", "a.vvp", src], w)
        if rc == 0:
            rc2, o2 = sh(["vvp", "-n", "a.vvp"], w)
            out.append(f"=== iverilog run rc={rc2}{(' cmsgs: ' + o.strip()[:200]) if o.strip() else ''}\n{clean(o2, w)}")
        else:
            out.append(f"=== iverilog compile rc={rc}\n" + "\n".join(clean(o, w).splitlines()[:6]))
            rc, o = sh([SV2V, src], w)
            if rc == 0:
                open(os.path.join(w, "s2v.v"), "w").write(o)
                rc1, o1 = sh(["iverilog", "-g2012", "-o", "b.vvp", "s2v.v"], w)
                if rc1 == 0:
                    rc2, o2 = sh(["vvp", "-n", "b.vvp"], w); out.append(f"=== sv2v->iverilog run rc={rc2}\n{clean(o2, w)}")
                else:
                    out.append(f"=== sv2v->iverilog compile rc={rc1}\n" + "\n".join(o1.splitlines()[:6]))
            else:
                out.append(f"=== sv2v rc={rc}\n" + "\n".join(o.splitlines()[:6]))
        rc, o = sh(["verilator", "--binary", "--timing", "--assert", "-Wno-fatal", "-Wno-lint", "-Wno-style", "--top-module", "top", "-Mdir", "obj", src], w, 300)
        if rc == 0:
            rc2, o2 = sh([os.path.join(w, "obj", "Vtop")], w)
            out.append(f"=== verilator run rc={rc2}\n{clean(o2, w)}")
        else:
            out.append(f"=== verilator compile rc={rc}\n" + "\n".join(clean(o, w).splitlines()[:8]))
        txt = "\n".join(out) + "\n"
        open(os.path.join(outdir, base + ".out"), "w").write(txt)
        summ.append(f"{base}: PRE2==POSTA {pre == pa} POSTA==POSTC {pa == pc} split {split} vcdAC {vac} stagedC {sc == pc['native']}")
        print(summ[-1], flush=True)
    open(os.path.join(outdir, "SUMMARY.txt"), "a").write("\n".join(summ) + "\n")
main()
