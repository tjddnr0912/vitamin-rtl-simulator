import re,sys
# usage: ptsub.py el2_param.vh file...   replace pt.FIELD with its decimal value
vals={}
for l in open(sys.argv[1]):
    m=re.match(r"\s*(?://\s*)?([A-Z0-9_]+)\s*:\s*(\d+)'h([0-9A-Fa-f]+)",l)
    if m: vals[m.group(1)]=int(m.group(3),16)
for f in sys.argv[2:]:
    s=open(f).read()
    n=[0]
    def r(m):
        k=m.group(1)
        if k in vals: n[0]+=1; return str(vals[k])
        return m.group(0)
    s2=re.sub(r"\bpt\.([A-Z0-9_]+)\b",r,s)
    open(f,"w").write(s2); print(f, n[0])
