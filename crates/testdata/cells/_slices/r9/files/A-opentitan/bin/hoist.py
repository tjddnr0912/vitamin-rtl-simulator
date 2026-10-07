# usage: hoist.py <in.sv> <out.sv>  -- hoist anonymous nested `struct packed {..} m;` members into named typedefs (workaround W-R02)
import sys, re
src = open(sys.argv[1]).read()
def match_brace(t, i):  # t[i] == '{'
    d = 0
    for j in range(i, len(t)):
        if t[j] == '{': d += 1
        elif t[j] == '}':
            d -= 1
            if d == 0: return j
    raise ValueError('unbalanced')
def hoist_body(body, prefix, out_defs):
    res = ''; i = 0
    pat = re.compile(r'\b(struct|union)\s+packed\s*(signed\s*|unsigned\s*)?\{')
    while True:
        m = pat.search(body, i)
        if not m: res += body[i:]; break
        ob = m.end() - 1; cb = match_brace(body, ob)
        tail = re.match(r'\s*((?:\[[^\]]*\]\s*)*)(\w+)\s*;', body[cb+1:])
        if not tail: res += body[i:cb+1]; i = cb+1; continue
        dims, mem = tail.group(1), tail.group(2)
        tname = f'{prefix}__{mem}_t'
        inner = hoist_body(body[ob+1:cb], tname[:-2], out_defs)
        out_defs.append(f'typedef {m.group(1)} packed {m.group(2) or ""}{{{inner}}} {tname};\n')
        res += body[i:m.start()] + f'{tname} {dims}{mem};'
        i = cb + 1 + tail.end()
    return res
out = ''; i = 0; n = 0
pat = re.compile(r'\btypedef\s+(struct|union)\s+packed\s*(signed\s*|unsigned\s*)?\{')
while True:
    m = pat.search(src, i)
    if not m: out += src[i:]; break
    ob = m.end() - 1; cb = match_brace(src, ob)
    name = re.match(r'\s*((?:\[[^\]]*\]\s*)*)(\w+)\s*;', src[cb+1:]).group(2)
    defs = []
    body = hoist_body(src[ob+1:cb], name, defs)
    n += len(defs)
    out += src[i:m.start()] + ''.join('  ' + d for d in defs) + src[m.start():ob+1] + body + '}'
    i = cb + 1
open(sys.argv[2], 'w').write(out)
print(f'hoisted={n}', file=sys.stderr)
