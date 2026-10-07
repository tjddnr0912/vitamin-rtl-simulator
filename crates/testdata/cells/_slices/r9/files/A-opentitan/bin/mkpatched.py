# usage: mkpatched.py <OT> <R> <filelist...>   rebuild R/patched overlay for every listed file that needs a workaround
import sys, os, re, subprocess, importlib.util
OT, R = sys.argv[1], sys.argv[2]; files = set()
for fl in sys.argv[3:]:
    files |= set(l.strip() for l in open(fl) if l.strip())
spec = importlib.util.spec_from_file_location('patches', R+'/bin/patches.py'); pm = importlib.util.module_from_spec(spec); spec.loader.exec_module(pm)
byfile = {}
for rid, rel, old, new in pm.P: byfile.setdefault(OT+'/'+rel, []).append((rid, old, new))
log = []
import glob
files |= set(glob.glob(OT+'/hw/ip/prim/rtl/*.sv*')) | set(glob.glob(OT+'/hw/dv/sv/dv_utils/*.sv*'))
for f in sorted(files | set(byfile)):
    if not os.path.exists(f): continue
    t = open(f).read(); o = t
    if f.endswith('_reg_pkg.sv') and re.search(r'\{\s*(struct|union)\s+packed', t):
        tmp = R+'/tmp/_h.sv'; subprocess.run(['python3','-I',R+'/bin/hoist.py',f,tmp],check=True,capture_output=True); t = open(tmp).read(); log.append(('R02', f))
    def _rep(m):
        n = m.group(2)
        if not n.isdigit():
            d = os.path.dirname(f); pk = [x for x in os.listdir(d) if x.endswith('_reg_pkg.sv')]
            n = None
            for x in pk:
                mm = re.search(r'parameter\s+int\s+NumRegs\s*=\s*(\d+)', open(os.path.join(d,x)).read())
                if mm: n = mm.group(1)
            if n is None: return m.group(0)
        return "'{" + ', '.join(['0']*int(n)) + "}"
    t2 = re.sub(r"'\{(\w+::)?(NumRegs|\d+)\{0\}\}", _rep, t)
    if t2 != t: t = t2; log.append(('R06', f))
    t2 = re.sub(r'\btop_racl_pkg::racl_policy_vec_t\b', 'top_racl_pkg::racl_policy_t [top_racl_pkg::NrRaclPolicies-1:0]', t)
    if t2 != t: t = t2; log.append(('R04', f))
    def _r05(m):
        a, b = m.group(2).strip(), m.group(3).strip()
        if re.fullmatch(r"[0-9]+", a) and re.fullmatch(r"[0-9]+", b): return m.group(0)
        return f"{m.group(1)}[({a}) -: (({a})-({b})+1)]"
    t2 = re.sub(r'(\b[a-zA-Z_]\w*(?:\[[^\]]*\])?(?:\.[a-zA-Z_]\w*)+)\[([^\]:+\-]*[^\]:+]*?)(?<![+-]):([^\]]*)\]', _r05, t.replace('::','\x01')).replace('\x01','::')
    if t2 != t: t = t2; log.append(('R05', f))
    for rid, frx, pat, rep in getattr(pm, 'RX', []):
        if re.fullmatch(frx, f):
            t2 = re.sub(pat, rep, t)
            if t2 != t: t = t2; log.append((rid, f))
    for rid, old, new in byfile.get(f, []):
        if old not in t: print('PATCH_MISS', rid, f, file=sys.stderr); continue
        t = t.replace(old, new); log.append((rid, f))
    if True:  # full mirror so `include resolves inside the overlay
        out = R+'/patched/'+os.path.relpath(f, OT); os.makedirs(os.path.dirname(out), exist_ok=True); open(out,'w').write(t)
for rid, f in log: print(rid, os.path.relpath(f, OT))
import subprocess as _sp
_r=_sp.run(['python3','-I',R+'/bin/mreg.py',R+'/patched'],capture_output=True,text=True); print(_r.stdout, end=''); print(_r.stderr, end='', file=sys.stderr)
