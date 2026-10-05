import subprocess,re,sys,os
S='/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s580'
D=S+'/lens_diff3'; SV2V=S+'/sv2v/sv2v-macOS/sv2v'
def sh(cmd,timeout=300):
    try:
        r=subprocess.run(cmd,shell=True,capture_output=True,text=True,cwd=D,timeout=timeout); return r.stdout+r.stderr,r.returncode
    except subprocess.TimeoutExpired: return 'TIMEOUT',-9
def parse(t):
    m={};codes=[];err=None
    for ln in t.split('\n'):
        mm=re.match(r'^([A-Z][A-Za-z0-9_]*) (.*)$',ln)
        if mm and not ln.startswith(('VCD','VITA')): m.setdefault(mm.group(1),mm.group(2).strip())
        codes+=re.findall(r'(?:error|warning)\[(VITA-[EW]\d+)\]',ln)
        if err is None and re.search(r'error|sorry|%Error|TIMEOUT|panic',ln): err=ln.strip()[:230]
    return m,codes,err
f=sys.argv[1]; sets=sys.argv[2].split(';'); flags=sys.argv[3].split(',') if len(sys.argv)>3 else []
b0=os.path.splitext(os.path.basename(f))[0]
for s in sets:
    defs='' if s.strip()=='-' else s.strip()
    tag=b0+('_'+defs.replace('-D','').replace(' ','_') if defs else '')
    cols={}
    for v in ['pre','post','post2','post3']: cols[v]=sh(f'{S}/{v}/vita {defs} {f}')
    if 'q' in flags:
        cols['post2q']=sh(f'{S}/post2/vita -DIV {defs} {f}'); cols['post3q']=sh(f'{S}/post3/vita -DIV {defs} {f}')
    if 'iv' in flags: cols['iv']=sh(f'iverilog -g2012 -DIV {defs} -o {tag}.vvp {f} && vvp -n {tag}.vvp')
    if 'sv' in flags: cols['sv']=sh(f'{SV2V} {defs} {f} > {tag}.sv2v.v && iverilog -g2012 -o {tag}.svvp {tag}.sv2v.v && vvp -n {tag}.svvp')
    if 'vl' in flags: cols['vl']=sh(f'rm -rf {tag}.obj; verilator --binary -Wno-fatal -Wno-lint -Wno-style --top-module t {defs} {f} -Mdir {tag}.obj > {tag}.vlb.txt 2>&1 && {tag}.obj/Vt',600)
    P={k:parse(t) for k,(t,rc) in cols.items()}
    for k,(t,rc) in cols.items(): open(f'{D}/{tag}.{k}.txt','w').write(t+f'\nrc={rc}\n')
    print(f'== {tag}: '+' '.join(f'{k}:rc{cols[k][1]}'+('/'+','.join(sorted(set(P[k][1]))) if P[k][1] else '') for k in cols))
    for k in cols:
        if cols[k][1]!=0 and P[k][2]: print(f'   [{k}] {P[k][2]}')
    keys=[]
    for k in cols:
        for kk in P[k][0]:
            if kk not in keys: keys.append(kk)
    hdr=list(cols); print('   id | '+' | '.join(hdr)+' | flags')
    for kk in keys:
        g=lambda c: P[c][0].get(kk,'-')
        fl=[]
        if g('post2')!=g('post3'): fl.append('D23')
        for o in ('iv','sv','vl'):
            ref='post3q' if (o=='iv' and 'post3q' in cols) else 'post3'
            if o in cols and g(o)!='-' and g(ref)!=g(o): fl.append('!'+o)
        if 'post3q' in cols and g('post3')!=g('post3q'): fl.append('q!')
        print(f'   {kk} | '+' | '.join(g(c) for c in hdr)+' | '+','.join(fl))
