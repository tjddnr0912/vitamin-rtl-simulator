import os,re,glob,sys
d=sys.argv[1]; tags=sys.argv[2].split(',')
def norm(p):
    if not os.path.exists(p): return '<missing>'
    out=[]
    for l in open(p,errors='replace'):
        l=l.rstrip()
        if re.search(r'W1017|^simulation ended|^errors=|^\s*$|^rc=0$',l): continue
        m=re.match(r'^(?:\S+:\d+:\d+: )?(error|fatal)\[(VITA-\S+)\]',l)
        if m: out.append('!'+m.group(2)); continue
        if re.match(r'^(?:\S+:\d+:\d+: )?(warning|note|info)\[',l): continue
        if l.startswith('rc='): continue
        out.append(l)
    return ' | '.join(sorted(set(out)))
for f in sorted(glob.glob(d+'/*.sv')):
    b=f[:-3]
    print(f"{os.path.basename(b):10} "+"  ".join(f"{t}={norm(b+'.'+t):26}" for t in tags))
