#!/usr/bin/env python3
"""Step-6 sweep: PRE vs POST x {native,interp,vm} one-shot; POST staged vs one-shot (native);
run.json subroutines/native/codegen PRE vs POST. Writes sweep.txt, prints a summary."""
import os, subprocess, sys, json, shutil, glob, re
S = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
PRE, POST = os.path.join(S, "pre", "vita"), os.path.join(S, "post", "vita")
ENV = dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")
W = os.path.join(S, "m", "sw"); shutil.rmtree(W, ignore_errors=True); os.makedirs(W)
cells = sorted(glob.glob(os.path.join(S, "c", "*.sv")) + glob.glob(os.path.join(S, "plan", "*.sv"))
               + [os.path.join(S, "m", f) for f in ("p0_chain.sv", "ctl_else.sv")]
               + [os.path.join(S, "j", f) for f in ("J1.sv", "J1_H.sv")])
def sh(cmd, cwd):
    try:
        p = subprocess.run(cmd, cwd=cwd, env=ENV, stdout=subprocess.PIPE, stderr=subprocess.PIPE, timeout=120)
        return p.returncode, p.stdout.decode("utf-8", "replace"), p.stderr.decode("utf-8", "replace")
    except subprocess.TimeoutExpired:
        return "TIMEOUT", "", ""
def clean(s):
    return "\n".join(l for l in s.splitlines() if "VITA-W1017" not in l and not l.startswith("errors="))
rep = []; summ = {"changed": [], "unchanged": [], "staged_mismatch": [], "route_mismatch": [], "backend_split_post": [], "timeout": []}
for c in cells:
    base = os.path.splitext(os.path.basename(c))[0]
    key = os.path.basename(os.path.dirname(c)) + "/" + base
    d = os.path.join(W, key.replace("/", "_")); os.makedirs(d)
    shutil.copy(c, os.path.join(d, base + ".sv"))
    res = {}
    for tag, b in (("PRE", PRE), ("POST", POST)):
        for be in ("native", "interp", "vm"):
            rc, o, e = sh([b, "--backend", be, "-o", f"{tag}_{be}.vcd", base + ".sv"], d)
            res[(tag, be)] = f"rc={rc}\n--stdout\n{o}--stderr\n{clean(e)}"
            if rc == "TIMEOUT": summ["timeout"].append(f"{key} {tag} {be}")
    changed = [be for be in ("native", "interp", "vm") if res[("PRE", be)] != res[("POST", be)]]
    post_split = not (res[("POST", "native")] == res[("POST", "interp")] == res[("POST", "vm")])
    if post_split: summ["backend_split_post"].append(key)
    (summ["changed"] if changed else summ["unchanged"]).append(key + (f" [{','.join(changed)}]" if changed and len(changed) < 3 else ""))
    # staged (POST)
    rc1, o1, e1 = sh([POST, "vcmp", base + ".sv"], d)
    rc2, o2, e2 = sh([POST, "velab", base + ".vu"], d)
    rc3, o3, e3 = sh([POST, "vrun", "-o", "st.vcd", base + ".velab"], d)
    one = res[("POST", "native")]
    st = f"rc={rc3}\n--stdout\n{o3}--stderr\n{clean(e3)}"
    # one-shot stderr also carries vcmp/velab-stage diagnostics; compare the W4031 lines and stdout
    def w31(s): return [l for l in s.splitlines() if "W4031" in l]
    def so(s): return s.split("--stderr")[0]
    st_ok = (rc1, rc2) == (0, 0) and w31(st) == w31(one) and so(st) == so(one)
    if not st_ok: summ["staged_mismatch"].append(f"{key} vcmp={rc1} velab={rc2} vrun={rc3}")
    # run.json
    rj = {}
    for tag, b in (("PRE", PRE), ("POST", POST)):
        od = os.path.join(d, f"obs_{tag}")
        sh([b, "--obs-dir", od, "-o", f"obs_{tag}.vcd", base + ".sv"], d)
        try:
            j = json.load(open(os.path.join(od, "run.json")))
            rj[tag] = {k: j.get(k) for k in ("subroutines", "native", "backend", "codegen", "status", "exit_code")}
            for it in (rj[tag]["subroutines"] or {}).get("items", []): it.pop("decl_file", None)
        except Exception as ex:
            rj[tag] = f"no run.json: {ex}"
    if rj["PRE"] != rj["POST"]:
        diffk = [k for k in ("subroutines", "native", "backend", "codegen", "status", "exit_code")
                 if not isinstance(rj["PRE"], dict) or rj["PRE"].get(k) != rj["POST"].get(k)]
        summ["route_mismatch"].append(f"{key} keys={diffk}")
    rep.append(f"##### {key}  changed={changed or '-'} post_backend_split={post_split} staged_ok={st_ok} runjson_eq={rj['PRE'] == rj['POST']}")
    if changed:
        for be in ("native",) if not post_split else ("native", "interp", "vm"):
            rep.append(f"=== PRE {be}\n{res[('PRE', be)]}\n=== POST {be}\n{res[('POST', be)]}")
    if not st_ok:
        rep.append(f"=== staged vrun\n{st}\n=== one-shot native\n{one}")
open(os.path.join(S, "m", "sweep.txt"), "w").write("\n".join(rep) + "\n")
print(f"cells={len(cells)}")
for k, v in summ.items():
    print(f"{k} ({len(v)}): " + "; ".join(v))
