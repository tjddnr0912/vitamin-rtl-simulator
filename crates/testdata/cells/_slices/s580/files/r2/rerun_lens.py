#!/usr/bin/env python3
# rerun_lens.py <new-vita> <tag> : re-run every lens probe variant with s580/post/vita (must reproduce the
# lens' recorded POST text) and with <new-vita>; write r2/lens/<variant>.<tag>.txt and report changes.
import subprocess, sys, os, re, itertools
S='/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad/s580'
NEW=sys.argv[1]; TAG=sys.argv[2]
OUT=f'{S}/r2/lens'
def run(binary, args, f, cwd):
    p=subprocess.run([binary]+args+[f], capture_output=True, text=True, cwd=cwd)
    return p.stdout+p.stderr+f"rc={p.returncode}\n"
def norm(t):
    return "\n".join(l for l in t.splitlines() if 'W1017' not in l and not l.startswith('errors=') and 'warnings=' not in l)
variants=[]
# lens_snd: plain run (+ NO_INSIDE twin is iverilog-only)
for f in sorted(os.listdir(f'{S}/lens_snd/probes')):
    if f.endswith('.sv'):
        variants.append(('snd',f[:-3],f'{S}/lens_snd/probes',f,[] ,'post'))
cands={'B':['-DB'],'RL':['-DRL'],'FIN':['-DFIN'],'FJ':['-DFJ'],'FRC':['-DFRC'],'IFC':['-DIFC'],'NOP':['-DNOP'],
       'LETT':['-DLETT'],'ARR':['-DARR'],'K':None}
D=f'{S}/lens_diff'
for f in sorted(os.listdir(D)):
    m=re.match(r'^(P\d+[a-z]?)(?:_(\w+))?\.post\.txt$', f)
    if not m: continue
    base, suf = m.group(1), m.group(2)
    src=f'{base}.sv'
    if not os.path.exists(f'{D}/{src}'): continue
    variants.append(('diff', f[:-len('.post.txt')], D, src, suf, 'post'))
results=[]
defs_all=['-DB','-DRL','-DFIN','-DFJ','-DFRC','-DIFC','-DNOP','-DLETT','-DARR','-DNOGC']+[f'-DK{i}' for i in range(1,10)]
for lens,name,d,src,suf,_ in variants:
    rec=open(f'{d}/{name}.post.txt').read() if lens=='diff' else open(f'{d}/{name}.post.txt').read()
    # find define set reproducing the recorded POST text
    tries=[[]]
    if lens=='diff' and suf:
        tries=[]
        sufs={'b':['-DB'],'r':['-DRL'],'n':['-DNOP'],'l':['-DLETT'],'FIN':['-DFIN'],'FJ':['-DFJ'],'FRC':['-DFRC'],'IFC':['-DIFC'],'all':['-DFIN','-DFJ','-DFRC','-DIFC']}
        if suf in sufs: tries.append(sufs[suf])
        if re.match(r'K\d',suf): tries.append([f'-D{suf}'])
        tries += [[x] for x in defs_all] + [['-DLETT','-DARR'],['-DFIN','-DFJ','-DFRC','-DIFC']]
    found=None
    for t in tries:
        o=run(f'{S}/post/vita', t, src, d)
        if norm(o)==norm(rec):
            found=t; break
    if found is None:
        results.append((name,'NOREPRO',None)); continue
    o_post=run(f'{S}/post/vita', found, src, d)
    o_new=run(NEW, found, src, d)
    open(f'{OUT}/{lens}_{name}.post.txt','w').write(o_post)
    open(f'{OUT}/{lens}_{name}.{TAG}.txt','w').write(o_new)
    q=None
    if lens=='diff':
        oq_post=run(f'{S}/post/vita', found+['-DIV'], src, d); oq_new=run(NEW, found+['-DIV'], src, d)
        open(f'{OUT}/{lens}_{name}.postq.txt','w').write(oq_post); open(f'{OUT}/{lens}_{name}.{TAG}q.txt','w').write(oq_new)
        q = norm(oq_post)!=norm(oq_new)
    results.append((name, 'changed' if norm(o_post)!=norm(o_new) else 'same', q, found))
for r in results: print(r)
