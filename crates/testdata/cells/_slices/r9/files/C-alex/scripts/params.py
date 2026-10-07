import re,sys,os
# params.py <file.v> <module>: header parameters "NAME = default" (one per line)
t=open(sys.argv[1]).read(); t=re.sub(r'/\*.*?\*/','',t,flags=re.S); t=re.sub(r'//[^\n]*','',t)
m=re.search(r'module\s+'+sys.argv[2]+r'\s*#\s*\((.*?)\)\s*\(',t,flags=re.S)
if not m: sys.exit(0)
body=m.group(1); out=[]; depth=0; cur=''
for ch in body:
    if ch in '([{': depth+=1
    if ch in ')]}': depth-=1
    if ch==',' and depth==0: out.append(cur); cur=''
    else: cur+=ch
out.append(cur)
for p in out:
    p=' '.join(p.split()); p=re.sub(r'^parameter\s+(integer\s+)?','',p)
    if p: print(p)
