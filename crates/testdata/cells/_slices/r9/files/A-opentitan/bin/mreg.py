# Workaround W-R12: flatten `mreg_t [N-1:0] m;` members of <ip> reg structs into `logic [N*W-1:0] m;`
# and rewrite `<..reg2hw..|..hw2reg..>.m[IDX](.f1(.f2)?)?` accesses into part-selects. In place on the overlay.
import sys, re, glob, os
root = sys.argv[1]
info = {}
ACC = {}
for _f in glob.glob(root + '/hw/**/*.sv', recursive=True):
    for _m in re.finditer(r'(\b\w*(?:reg2hw|hw2reg)\w*)\.(\w+)\s*\[([^\]\[]+)\]', open(_f).read()):
        _k = ('hw2reg' if 'hw2reg' in _m.group(1) else 'reg2hw', _m.group(2))
        ACC.setdefault(_k, set()).add(bool(re.fullmatch(r'\s*\d+\s*', _m.group(3))))
    for _m in re.finditer(r'(\b\w*(?:reg2hw|hw2reg)\w*)\.(\w+)\b(?!\s*[\[.])', open(_f).read()):
        _k = ('hw2reg' if 'hw2reg' in _m.group(1) else 'reg2hw', _m.group(2))
        ACC.setdefault(_k, set()).add(False)
def layout(defs, ty):
    # returns (W, {path: (off, w)}) for a packed struct typedef of logic / nested typedef members
    body = defs.get(ty)
    if body is None: return None
    items = []
    for x in re.sub(r'//[^\n]*', '', body).split(';'):
        x = x.strip()
        if not x: continue
        m = re.match(r'logic\s*(\[\s*(\d+)\s*:\s*(\d+)\s*\])?\s*(\w+)$', x)
        if m: items.append((m.group(4), int(m.group(2)) - int(m.group(3)) + 1 if m.group(1) else 1, {})); continue
        m = re.match(r'(\w+_t)\s+(\w+)$', x)
        if m:
            sub = layout(defs, m.group(1))
            if sub is None: return None
            items.append((m.group(2), sub[0], sub[1])); continue
        return None
    W = sum(w for _, w, _ in items); off = W; paths = {}
    for n, w, sub in items:
        off -= w; paths[n] = (off, w)
        for p, (o2, w2) in sub.items(): paths[n + '.' + p] = (off + o2, w2)
    return W, paths
for f in glob.glob(root + '/hw/**/*_reg_pkg.sv', recursive=True):
    ip = os.path.basename(f)[:-len('_reg_pkg.sv')]
    t = open(f).read(); defs = {m.group(2): m.group(1) for m in re.finditer(r'typedef struct packed \{(.*?)\}\s*(\w+);', t, re.S)}
    def rep(m):
        ty, n_hi, n_lo, mem = m.group(2), int(m.group(3)), int(m.group(4)), m.group(5)
        L = layout(defs, ty)
        if L is None: print('R12 SKIP', ip, ty, file=sys.stderr); return m.group(0)
        kind = 'hw2reg' if '_hw2reg_' in ty else 'reg2hw'
        if ACC.get((kind, mem), {True}) == {True}:
            info[(ip, kind, mem)] = ('SEP', n_hi, n_lo)
            return ''.join(f'{m.group(1)}{ty} {mem}__{k};' for k in range(n_hi, n_lo - 1, -1))
        info[(ip, kind, mem)] = (n_hi - n_lo + 1, L[0], L[1])
        return f'{m.group(1)}logic [{(n_hi - n_lo + 1) * L[0] - 1}:0] {mem};'

    t2 = re.sub(r'^(\s*)(\w+_t)\s*\[\s*(\d+)\s*:\s*(\d+)\s*\]\s*(\w+)\s*;', rep, t, flags=re.M)
    if t2 != t: open(f, 'w').write(t2); print('R12 pkg', ip)
ips = set(k[0] for k in info)
for f in glob.glob(root + '/hw/**/*.sv', recursive=True):
    t = open(f).read(); o = t
    for ip in ips:
        if ip + '_reg' not in t and os.path.basename(os.path.dirname(os.path.dirname(f))) != ip: continue
        def acc(m):
            pfx, mem, idx, path = m.group(1), m.group(2), m.group(3).strip(), (m.group(4) or '').lstrip('.')
            k = (ip, 'hw2reg' if 'hw2reg' in pfx else 'reg2hw', mem)
            if k not in info: return m.group(0)
            if info[k][0] == 'SEP':
                return f"{pfx}.{mem}__{idx}" + (('.' + path) if path else '')
            N, W, paths = info[k]
            # longest matching field path prefix
            parts = path.split('.') if path else []
            use, rest = None, []
            for i in range(len(parts), 0, -1):
                if '.'.join(parts[:i]) in paths: use = '.'.join(parts[:i]); rest = parts[i:]; break
            if path and use is None: return m.group(0)
            off, w = paths[use] if use else (0, W)
            tail = ('.' + '.'.join(rest)) if rest else ''
            if re.fullmatch(r'\d+', idx):
                lo = int(idx) * W + off; return f'{pfx}.{mem}[{lo + w - 1}:{lo}]{tail}'
            return f'{pfx}.{mem}[({idx})*{W}+{off} +: {w}]{tail}'
        t = re.sub(r'(\b\w*(?:reg2hw|hw2reg)\w*)\.(\w+)\s*\[([^\]\[]+)\]((?:\.\w+)*)', acc, t)
    if t != o: open(f, 'w').write(t); print('R12 acc', os.path.relpath(f, root))
