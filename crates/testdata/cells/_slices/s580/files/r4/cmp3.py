#!/usr/bin/env python3
# cmp3.py <new-vita> <out.txt> : every probe variant, classify NEW vs PRE and POST3 (byte compare, W1017 and path prefix stripped)
import subprocess, sys, os, re, glob
SP='/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad'
S=f'{SP}/s580'
NEW=sys.argv[1]; OUT=sys.argv[2]
PRE=f'{S}/pre/vita'; P3=f'{S}/post3/vita'
pats=['census/*.sv','census/e/*.sv','census/c/*.sv','census/cq/*.sv','census/z/*.sv','fx/*.sv','probes/*.sv','ident/I*.sv',
      'r2/*.sv','r2/fx2/*.sv','r2/r26/*.sv','r4/fx/*.sv',
      'lens_snd/probes/*.sv','lens_diff/*.sv','lens_diff2/*.sv','lens_snd2/probes/*.sv','lens_diff3/*.sv','lens_snd3/p/*.sv']
files=[]
for p in pats: files+=sorted(glob.glob(f'{S}/{p}'))
files.append(f'{SP}/f2/w65.sv')
def run(b,args,f):
    p=subprocess.run([b]+args+[os.path.basename(f)],capture_output=True,text=True,cwd=os.path.dirname(f))
    out=[]
    for l in (p.stdout+p.stderr).splitlines():
        if 'W1017' in l: continue
        out.append(re.sub(r'^\S*/([^/\s]+\.sv):', r'\1:', l))
    return out+[f'rc={p.returncode}']
res={'both':0,'pre':0,'post3':0,'neither':0}
lines=[]
for f in files:
    src=open(f).read()
    defs=sorted(set(re.findall(r'`(?:ifdef|ifndef|elsif)\s+(\w+)',src)))
    base=[[]]+[[f'-D{d}'] for d in defs if d not in ('IV','NO_INSIDE')]
    variants=list(base)
    for extra in ('IV','NO_INSIDE'):
        if extra in defs: variants+= [v+[f'-D{extra}'] for v in base]
    for v in variants:
        a=run(PRE,v,f); b=run(P3,v,f); n=run(NEW,v,f)
        k=('both' if n==a and n==b else 'pre' if n==a else 'post3' if n==b else 'neither')
        res[k]+=1
        tag=os.path.relpath(f,SP)+' '+' '.join(v)
        lines.append(f'{k:8} {tag}')
        if k=='neither':
            sa=set(a); sb=set(b); sn=set(n)
            for l in n:
                if l not in sa or l not in sb:
                    lines.append(f'         NEW  {l[:170]}   [{"=PRE" if l in sa else "!PRE"} {"=POST3" if l in sb else "!POST3"}]')
            for l in a:
                if l not in sn: lines.append(f'         PRE  {l[:170]}')
            for l in b:
                if l not in sn: lines.append(f'         POST3 {l[:170]}')
open(OUT,'w').write('\n'.join(lines)+'\n'+str(res)+'\n')
print(res)
