import re,sys,json,subprocess,os
# variants.py <file.v> <module> <v1|v2|v3>  -> JSON dict of overrides for literal-default params ('' if none)
f,mod,v=sys.argv[1:4]
here=os.path.dirname(os.path.abspath(__file__))
lines=subprocess.run([sys.executable,'-I',os.path.join(here,'params.py'),f,mod],capture_output=True,text=True).stdout.splitlines()
P={}
for l in lines:
    k,_,d=l.partition(' = ')
    if re.fullmatch(r"\d+|\d*'[bdh][0-9a-fA-F_]+",d.strip()): P[k.strip()]=d.strip()
def num(d):
    m=re.fullmatch(r"(\d*)'([bdh])([0-9a-fA-F_]+)",d)
    if m: return int(m.group(3).replace('_',''),{'b':2,'d':10,'h':16}[m.group(2)])
    return int(d)
o={}
for k,d in P.items():
    n=num(d)
    if v=='v1':
        if k in('DATA_WIDTH','S_DATA_WIDTH') and n<=32: o[k]=64
        elif k=='M_DATA_WIDTH': o[k]=16
        elif k in('AXIS_PCIE_DATA_WIDTH',): o[k]=512
        elif k=='TLP_DATA_WIDTH' and n==256: o[k]=512
        elif k=='SEG_COUNT' and 'SEG_DATA_WIDTH' in P and P['SEG_DATA_WIDTH'] in('128','256'): o[k]=2; o['SEG_DATA_WIDTH']=256
        elif k in('IN_TLP_SEG_COUNT','TLP_SEG_COUNT') and 'TLP_DATA_WIDTH' in P: o[k]=2
        elif k.endswith('_ENABLE') and n in(0,1): o[k]=1
        elif re.search(r'(^|_)ID_WIDTH$',k): o[k]=5
        elif re.search(r'(^|_)DEST_WIDTH$',k): o[k]=3
        elif re.search(r'(^|_)USER_WIDTH$',k) and n<=8: o[k]=4
        elif k in('S_COUNT','M_COUNT','PORTS'): o[k]=3
        elif k in('FRAME_FIFO','DROP_BAD_FRAME','UPDATE_TID','TDEST_ROUTE','CTRL_OUT_EN','EXTEND_RAM_SEL','TLP_FORCE_64_BIT_ADDR','ARB_BLOCK'): o[k]=1
        elif k in('ARB_TYPE_ROUND_ROBIN','ARB_LSB_HIGH_PRIORITY','USE_AXI_ID','CHECK_BUS_NUMBER','LSB_HIGH_PRIORITY'): o[k]=1-n
        elif k=='WIDTH' and mod=='priority_encoder': o[k]=13
    elif v=='v2':
        if k in('DATA_WIDTH','S_DATA_WIDTH') and n<=32: o[k]=16
        elif k=='M_DATA_WIDTH': o[k]=64
        elif k in('AXIS_PCIE_DATA_WIDTH',): o[k]=64
        elif k=='TLP_DATA_WIDTH' and n==256: o[k]=128
        elif k.endswith('_ENABLE') and n in(0,1): o[k]=0
        elif re.search(r'(^|_)ID_WIDTH$',k): o[k]=12
        elif re.search(r'(^|_)DEST_WIDTH$',k): o[k]=9
        elif re.search(r'(^|_)USER_WIDTH$',k) and n<=8: o[k]=7
        elif k in('S_COUNT','M_COUNT','PORTS'): o[k]=5
        elif k in('REG_TYPE','M_REG_TYPE'): o[k]=1
        elif k=='S_REG_TYPE': o[k]=2
        elif k=='LENGTH': o[k]=4
        elif k=='RAM_PIPELINE': o[k]=3
        elif k in('ARB_BLOCK','ARB_BLOCK_ACK'): o[k]=1-n
        elif k=='WIDTH' and mod=='priority_encoder': o[k]=1
        elif k=='FILTER_LEN': o[k]=2
        elif k=='APPEND_ZERO': o[k]=0
    elif v=='v3':
        if k in('S_COUNT','M_COUNT','PORTS'): o[k]=1
print(json.dumps(o) if o else '')
