import re,sys,os,json
# usage: closure.py <rtl_dir>  -> json {module: [files...]}
d=sys.argv[1]
defs={}
src={}
for f in sorted(os.listdir(d)):
    if not f.endswith('.v'): continue
    t=open(os.path.join(d,f)).read()
    t=re.sub(r'/\*.*?\*/','',t,flags=re.S); t=re.sub(r'//[^\n]*','',t)
    src[f]=t
    for m in re.findall(r'^\s*module\s+(\w+)',t,flags=re.M): defs[m]=f
names=set(defs)
deps={}
for f,t in src.items():
    used=set()
    for n in names:
        if re.search(r'(?<![\w.$])'+n+r'(\s*#|\s+\w+\s*(\[[^\]]*\]\s*)?\()',t) and defs[n]!=f: used.add(defs[n])
    deps[f]=used
out={}
for m,f in sorted(defs.items()):
    seen=[f]; st=[f]
    while st:
        x=st.pop()
        for y in sorted(deps[x]):
            if y not in seen: seen.append(y); st.append(y)
    out[m]=seen
# extern: instantiations of unknown modules
ext={}
for f,t in src.items():
    for mm in re.finditer(r'^\s*([A-Za-z_]\w*)\s*(#\s*\(|[A-Za-z_]\w*\s*\()',t,flags=re.M):
        n=mm.group(1)
        if n in names or n in ('module','always','assign','if','for','case','function','task','begin','end','else','wire','reg','input','output','inout','localparam','parameter','integer','generate','initial','genvar','return','while','repeat','casez','casex','default'): continue
        ext.setdefault(f,set()).add(n)
json.dump({'closure':out,'extern':{k:sorted(v) for k,v in ext.items()}},sys.stdout,indent=0)
