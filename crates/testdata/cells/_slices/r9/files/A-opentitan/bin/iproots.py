# per-IP root membership: (a) workaround log x file list (static), (b) diagnostic signatures in vita.log / vitap.log (dynamic)
import sys, os, glob, collections, re
R, OT = sys.argv[1], sys.argv[2]
pl = collections.defaultdict(set)
for l in open(R+'/patchlog.txt'):
    p = l.split()
    if len(p) < 2 or not p[0].startswith('R'): continue
    if p[0] == 'R12':
        if p[1] == 'pkg': pl['ip:'+p[2]].add('R04')
        continue
    pl[p[-1]].add(p[0])
SIG = [
 ('R02', r"expected a net/var type in a struct/union member, found keyword 'struct'"),
 ('R04', r"simple chained typedef alias|explicit data type on an array parameter|_reg_pkg\.sv:\d+:\d+: error\[VITA-E2002\] E-PARSE-UNEXPECTED-TOKEN: expected identifier, found '\['|ast_pkg\.sv:\d+:\d+: error.*found '\['|prim_sha2_pkg\.sv:144|spi_device\.sv:88:"),
 ('R05', r"packed-struct member part-select must be a constant range|on a packed-struct member WRITE|constant packed-struct member bit-select WRITE index"),
 ('R06', r"expected '\}' closing an assignment pattern, found '\{'"),
 ('R08', r"\$func\$assert_static_in_package"),
 ('R09', r"concatenation-target assignment"),
 ('R10', r"undefined name `(tl_i|tl_h_i|reg2hw|edn_i|keymgr_key_i|passthrough_i)\w*` is not a constant"),
 ('R11', r"is not allowed in a constant range bound"),
 ('R14', r"type parameter override"),
 ('R17', r"partial unpacked-array slice|must be connected to a declared array"),
 ('R19', r"spi_device_pkg\.sv:42\d:\d+: error.*found '::'"),
 ('R21', r"laid out per instance\), found (?!keyword 'endpackage')|whole-member (read|write)"),
 ('R23', r"adc_ctrl_core\.sv:106:"),
 ('R24', r"keymgr_dpe_pkg\.sv:427:"),
 ('R25', r"enum label `CMD_STS_UNDRIVEN` value is not a foldable"),
 ('R26', r"package parameter `(DecLc\w*|TokenIdxWidth)` value is not a foldable"),
 ('R27', r"whole unpacked array has no value|whole unpacked array cannot be the write target"),
 ('R28', r"enum label `(LcSt|LcCnt|Ownership)\w*` value is not a foldable"),
 ('R29', r"`TransTokenIdxMatrix` value is not a foldable"),
 ('R30', r"condition that holds a string literal"),
 ('R31', r"\$bits.*concatenation|parameter `SyncWidth` value is not a constant: the concatenation"),
]
for d in sorted(glob.glob(R+'/ips/*/files.txt')):
    ip = d.split('/')[-2]
    if ip in ('prims',): continue
    st = set(); dy = set()
    files = [f.strip() for f in open(d) if f.strip()]
    for f in files:
        rel = os.path.relpath(f, OT); st |= pl.get(rel, set())
        b = os.path.basename(rel)
        if b.endswith('_reg_pkg.sv'): st |= pl.get('ip:'+b[:-11], set())
    if any(f.endswith('prim_sparse_fsm_flop.sv') for f in files): st.add('R14')
    txt = ''
    for lg in ('vita.log', 'vitap.log'):
        p = os.path.join(os.path.dirname(d), lg)
        if os.path.exists(p): txt += open(p).read()
    for rid, rx in SIG:
        if re.search(rx, txt): dy.add(rid)
    print(f"{ip:12s} static={' '.join(sorted(st))} | diag={' '.join(sorted(dy))}")
