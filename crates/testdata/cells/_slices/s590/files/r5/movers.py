import subprocess, os, shutil, glob, re
S="/private/tmp/claude-501/-Users-seongwookjang-project-git-vitamin-rtl-simulator/d07b7e3f-083c-4d58-9fce-b42df268e9c8/scratchpad"
G=f"{S}/s590"; OUT=f"{G}/r5/movers"
env=dict(os.environ, DEVELOPER_DIR="/Library/Developer/CommandLineTools")
names=["pl_x3_tristate_bus","u20b","u20d","d19_delay_chain","e01_tri_md","r2a_md2_rev","r2b_md_mixed_rev","f06_held_delayed_after_t0","f07_rchain_random_probe","r3e_held_lanes_vcd"]
dirs=[f"{G}/plan/c5",f"{G}/plan/w",f"{G}/r1/diff/cells",f"{G}/r2/sound/newcells",f"{G}/r3/diff/cells",f"{G}/r3/sound/newcells"]
def sh(cmd,cwd,timeout=300):
    try:
        p=subprocess.run(cmd,cwd=cwd,env=env,capture_output=True,timeout=timeout); return p.returncode,(p.stdout+p.stderr).decode("utf-8","replace")
    except subprocess.TimeoutExpired: return "TIMEOUT",""
def clean(o): return "\n".join(l for l in o.splitlines() if not re.search(r"I2021|W1017|^errors=|S i m u|Verilator:|VCD info|\$finish called|Verilog \$finish|^- ",l))
for n in names:
    src=[os.path.join(d,n+".sv") for d in dirs if os.path.exists(os.path.join(d,n+".sv"))][0]
    w=f"{OUT}/w_{n}"; shutil.rmtree(w,ignore_errors=True); os.makedirs(w); shutil.copy(src,w); f=n+".sv"
    res={}
    for tag,b in (("PRE3",f"{G}/pre3/vita"),("post_d",f"{G}/post_d/vita"),("post_e",f"{G}/r5/dev/vita")):
        outs=[sh([b,"--backend",be,"-o",f"{tag}.vcd",f],w) for be in ("native","interp","vm")]
        same=all(o==outs[0] for o in outs)
        res[tag]=(outs[1], same)
    rc,o=sh(["iverilog","-g2012","-o","a.vvp",f],w)
    iv=sh(["vvp","-n","a.vvp"],w) if rc==0 else (rc,"compile: "+o.splitlines()[0] if o else "")
    rc,o=sh(["verilator","--binary","--timing","--assert","-Wno-fatal","-Wno-lint","-Wno-style","--Mdir","obj","--top-module","top",f],w,600)
    vl=sh([os.path.join(w,"obj","Vtop"),"+verilator+error+limit+1000"],w) if rc==0 else (rc,"compile failed")
    txt=[f"##### {n}  ({src})"]
    for tag in ("PRE3","post_d","post_e"):
        (rc,o),same=res[tag]; txt.append(f"=== {tag} rc={rc} native=interp=vm:{same}\n{clean(o)}")
    txt.append(f"=== iverilog rc={iv[0]}\n{clean(iv[1])}"); txt.append(f"=== verilator rc={vl[0]}\n{clean(vl[1])}")
    open(f"{OUT}/{n}.txt","w").write("\n".join(txt)+"\n")
    print("done",n,flush=True)
