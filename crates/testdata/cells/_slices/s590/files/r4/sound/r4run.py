import subprocess, os, sys, hashlib, shutil, glob
S="/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad"
R2=S+"/s590/r4/sound"
BIN={"PRE3":S+"/s590/pre3/vita","POSTD":S+"/s590/post_d/vita","POSTE":S+"/s590/post_e/vita"}
BE=["native","interp","vm"]
ENV=dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")
OUT=R2+"/out"
def sh(cmd, cwd, t=120):
    try:
        p=subprocess.run(cmd,cwd=cwd,env=ENV,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,timeout=t)
        return p.returncode, p.stdout.decode(errors="replace")
    except subprocess.TimeoutExpired:
        return "TIMEOUT", ""
def prep(w, src):
    shutil.rmtree(w, ignore_errors=True); os.makedirs(w); shutil.copy(src, w)
def vcds(w):
    return [f"VCD {os.path.basename(v)} md5={hashlib.md5(open(v,'rb').read()).hexdigest()}" for v in sorted(glob.glob(w+"/*.vcd"))]
def norm(o, w): return o.replace(w+"/","").replace(w,"")
os.makedirs(OUT, exist_ok=True)
for src in sys.argv[1:]:
    name=os.path.basename(src)[:-3]; base=R2+"/w/"+name; out=[f"##### {name}"]
    if os.environ.get("NO_ORACLE"): pass
    else:
     w=base+"/iv"; prep(w,src)
     rc,o=sh(["iverilog","-g2012","-o","a.out",name+".sv"],w)
     if rc!=0: out.append(f"=== iverilog compile rc={rc}\n{norm(o,w)[:1200]}")
     else:
         rc,o=sh(["vvp","-n","a.out"],w); out.append(f"=== iverilog run rc={rc}\n{norm(o,w)}")
     w=base+"/vl"; prep(w,src)
     rc,o=sh(["verilator","--binary","--timing","--assert","-Wno-fatal","-Wno-lint","-Wno-style","--Mdir","obj","-o","Vtop",name+".sv"],w,600)
     if rc!=0: out.append(f"=== verilator compile rc={rc}\n"+"\n".join(l for l in norm(o,w).splitlines() if "%Error" in l)[:1200])
     else:
         rc,o=sh([w+"/obj/Vtop"],w); out.append(f"=== verilator run rc={rc}\n{norm(o,w)}")
     shutil.rmtree(w+"/obj", ignore_errors=True)
    res={}
    for b,path in BIN.items():
        for be in BE:
            w=f"{base}/{b}_{be}"; prep(w,src); rc,o=sh([path,"--backend",be,name+".sv"],w)
            res[(b,be)]=f"rc={rc}\n{norm(o,w)}"+"".join("\n"+v for v in vcds(w))
    for b in BIN:
        vals=[res[(b,be)] for be in BE]
        if all(v==vals[0] for v in vals): out.append(f"=== {b} (native=interp=vm)\n{vals[0]}")
        else: out += [f"=== {b} {be} [BACKEND SPLIT]\n{res[(b,be)]}" for be in BE]
    ks=list(BIN); fl=[]
    for i in range(len(ks)):
        for j in range(i+1,len(ks)):
            fl.append(f"{ks[i]}=={ks[j]}:"+"".join("1" if res[(ks[i],be)]==res[(ks[j],be)] else "0" for be in BE))
    split=[b for b in BIN if len({res[(b,be)] for be in BE})>1]
    line=f"{name} "+" ".join(fl)+f" split={','.join(split) or '-'}"
    out.append("--- "+line); open(f"{OUT}/{name}.out","w").write("\n".join(out)+"\n"); print(line, flush=True)
