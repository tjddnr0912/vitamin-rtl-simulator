import sys,re
# cmpcols.py <dir> <tag>
d,tag=sys.argv[1:3]
tb=open(f'{d}/tb_{tag}.v').read()
cols=['cyc']+re.search(r'\$display\("[^"]*", tb_cyc((?:, \w+)*)\)',tb).group(1).split(', ')[1:]
a=[l.split() for l in open(f'{d}/{tag}.iv.tr')]; b=[l.split() for l in open(f'{d}/{tag}.vita.tr')]
st={}
for i,(x,y) in enumerate(zip(a,b)):
    for j,(u,v) in enumerate(zip(x,y)):
        if u!=v:
            s=st.setdefault(cols[j],[0,i,i,u,v]); s[0]+=1; s[2]=i
for k,s in st.items():
    u,v=s[3],s[4]
    print(f'  {k}: ndiff={s[0]} first={s[1]} last={s[2]} iv={u[:40]} vita={v[:40]}')
if len(a)!=len(b): print('  LEN', len(a), len(b))
