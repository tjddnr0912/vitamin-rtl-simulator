import sys, re, os, glob
# usage: deps.py <top_module> [extra_file...]  -> prints ordered file list (packages topo-sorted, then modules)
OT = sys.argv[1]; top = sys.argv[2]; extra = sys.argv[3:]
dirs = [OT+'/hw/ip/prim_generic/rtl']
for d in sorted(glob.glob(OT+'/hw/ip/*/rtl')):
    if any(x in d for x in ('prim_xilinx','prim_asap7','/rv_core_ibex/','prim_generic')): continue
    dirs.append(d)
dirs += sorted(glob.glob(OT+'/hw/top_earlgrey/ip_autogen/*/rtl'))
dirs += [OT+'/hw/top_earlgrey/rtl', OT+'/hw/top_earlgrey/rtl/autogen', OT+'/hw/top_earlgrey/ip/ast/rtl', OT+'/hw/vendor/lowrisc_ibex/rtl']
def strip(t):
    t = re.sub(r'/\*.*?\*/', ' ', t, flags=re.S)
    t = re.sub(r'//[^\n]*', ' ', t)
    return t
mods, pkgs, texts = {}, {}, {}
files = []
for d in dirs:
    for f in sorted(glob.glob(d+'/*.sv')): files.append(f)
for f in extra: files.insert(0, os.path.abspath(f))
for f in files:
    t = strip(open(f, errors='replace').read()); texts[f] = t
    for m in re.finditer(r'^\s*(module|interface|package)\s+(?:automatic\s+|static\s+)?(\w+)', t, re.M):
        kind, name = m.group(1), m.group(2)
        tbl = pkgs if kind == 'package' else mods
        if name not in tbl: tbl[name] = f
kw = set('if else for case begin end assign always always_ff always_comb always_latch initial generate wire logic reg input output inout module function task return typedef localparam parameter'.split())
def deps_of(f):
    t = texts[f]
    ms = set()
    for m in re.finditer(r'(?:^|[;\s])(\w+)\s*(?:#\s*\(|(?:\\\S+|\w+)\s*(?:\[[^\]]*\]\s*)*\()', t, re.M):
        n = m.group(1)
        if n in mods and n not in kw and mods[n] != f: ms.add(n)
    ps = set(re.findall(r'\b(\w+)\s*::', t)) & set(pkgs)
    if 'PRIM_FLOP_SPARSE_FSM' in t: ms.add('prim_sparse_fsm_flop')
    return ms, ps
want_m, want_p = set(), set()
todo = [top]
while todo:
    n = todo.pop()
    if n in want_m: continue
    if n not in mods: print('MISSING_MODULE', n, file=sys.stderr); continue
    want_m.add(n)
    ms, ps = deps_of(mods[n])
    todo += list(ms)
    for p in ps: want_p.add(p)
# package closure
ptodo = list(want_p)
while ptodo:
    p = ptodo.pop()
    _, ps = deps_of(pkgs[p])
    for q in ps:
        if q not in want_p: want_p.add(q); ptodo.append(q)
order = []
def visit(p, stack=()):
    if p in order: return
    _, ps = deps_of(pkgs[p])
    for q in sorted(ps):
        if q != p and q not in stack: visit(q, stack+(p,))
    order.append(p)
for p in sorted(want_p): visit(p)
seen = set(); out = []
for p in order:
    f = pkgs[p]
    if f not in seen: seen.add(f); out.append(f)
for m in sorted(want_m):
    f = mods[m]
    if f not in seen: seen.add(f); out.append(f)
print('\n'.join(out))
