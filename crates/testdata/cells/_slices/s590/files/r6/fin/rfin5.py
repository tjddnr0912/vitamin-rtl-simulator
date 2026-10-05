#!/usr/bin/env python3
"""rfin.py OUTDIR cell.sv... : PRE2 x3, POST x3 (one-shot), staged (separate bins) PRE2/POST,
POST no-default-features (native), POST jit (VITA_JIT=1) x3. Same section format as g/r.py."""
import os, re, subprocess, sys, shutil
S = "/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad"
PRE = f"{S}/s590/pre3/vita"; POST = f"{S}/s590/post_e/vita"; POSTA = f"{S}/s590/post_d/vita"
PRES = f"{S}/s590/pre3/sep"; POSTS = f"{S}/s590/post_e/sep"
NODEF = f"{S}/s590/post_e/nodef/vita"; JIT = f"{S}/s590/post_e/jit/vita"
ENV = dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")
WD = f"{S}/s590/r6/fin/w"
def sh(cmd, cwd, env=ENV):
    try:
        p = subprocess.run(cmd, cwd=cwd, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, timeout=120)
        return p.returncode, p.stdout.decode("utf-8", "replace")
    except subprocess.TimeoutExpired as e:
        return "TIMEOUT", (e.stdout or b"").decode("utf-8", "replace")
def clean(o, w):
    o = o.replace(w + "/", "")
    return "\n".join(l for l in o.splitlines() if "VITA-W1017" not in l)
def vita3(tag, b, f, w, out, env=ENV):
    res = {}
    for be in ("native", "interp", "vm"):
        rc, o = sh([b, "--backend", be, "-o", f"{tag}_{be}.vcd", f], w, env)
        res[be] = f"rc={rc}\n{clean(o, w)}"
    if res["native"] == res["interp"] == res["vm"]:
        out.append(f"=== vita {tag} (native=interp=vm)\n{res['native']}")
    else:
        for be in ("native", "interp", "vm"):
            out.append(f"=== vita {tag} {be} [BACKEND SPLIT]\n{res[be]}")
    return res
def staged(tag, d, src, base, w, out):
    sd = os.path.join(w, "st_" + tag); os.makedirs(sd, exist_ok=True); shutil.copy(os.path.join(w, src), sd)
    r1 = sh([f"{d}/vcmp", "-o", base + ".vu", src], sd)
    r2 = sh([f"{d}/velab", "-o", base + ".velab", base + ".vu"], sd)
    r3 = sh([f"{d}/vrun", "-o", "st.vcd", base + ".velab"], sd)
    out.append(f"=== staged {tag} vcmp={r1[0]} velab={r2[0]}\nrc={r3[0]}\n{clean(r3[1], sd)}")
    return f"rc={r3[0]}\n{clean(r3[1], sd)}"
def rt(t):
    # runtime view: drop compile-stage diagnostics (codes below 4000) and the summary counts
    return [l for l in t.splitlines() if not re.search(r"\[VITA-[A-Z][0-3]\d{3}\]", l) and not l.startswith("errors=")]
def main():
    outdir = sys.argv[1]; os.makedirs(outdir, exist_ok=True)
    for c in sys.argv[2:]:
        c = os.path.abspath(c); base = os.path.splitext(os.path.basename(c))[0]
        w = os.path.join(WD, base); shutil.rmtree(w, ignore_errors=True); os.makedirs(w)
        shutil.copy(c, os.path.join(w, base + ".sv")); src = base + ".sv"
        out = [f"##### {base}"]
        pre = vita3("PRE", PRE, src, w, out)
        post = vita3("POST", POST, src, w, out)
        posta = vita3("POSTA", POSTA, src, w, out)
        out.append(f"--- POSTA==POST (all backends): {posta == post}")
        out.append(f"--- PRE==POST (all backends): {pre == post}")
        sp = staged("PRE", PRES, src, base, w, out)
        sq = staged("POST", POSTS, src, base, w, out)
        out.append(f"--- staged POST == one-shot POST native (runtime lines): {rt(sq) == rt(post['native'])}  staged PRE == one-shot PRE native: {rt(sp) == rt(pre['native'])}")
        rc, o = sh([NODEF, "-o", "nodef.vcd", src], w)
        nd = f"rc={rc}\n{clean(o, w)}"
        out.append(f"=== nodef native\n{nd}")
        out.append(f"--- nodef == POST native: {nd == post['native']}")
        jenv = dict(ENV, VITA_JIT="1")
        j = vita3("JIT", JIT, src, w, out, jenv)
        out.append(f"--- JIT == POST (all backends): {j == post}")
        txt = "\n".join(out) + "\n"
        open(os.path.join(outdir, base + ".out"), "w").write(txt)
main()
