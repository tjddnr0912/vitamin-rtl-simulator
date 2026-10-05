import re,sys
# make ==? twins of single-element inside cells: <base>.sv -> <base>_wq.sv with ids suffixed q
for base in sys.argv[1:]:
    out=[]
    for l in open(base+'.sv'):
        m=re.match(r'^(\s*)(.*)\$display\("([A-Z][A-Za-z0-9]*) %b", (.*)\);$',l.rstrip('\n'))
        if m:
            ind,setup,cid,expr=m.groups()
            mm=re.fullmatch(r'(\(?)(!?)\(?(\w+) inside \{([^,\[\]]+)\}\)?',expr)
            mm=re.fullmatch(r'(\w+) inside \{([^-,\[\]{}][^,\[\]{}]*)\}',expr)
            if mm:
                out.append(f'{ind}{setup}$display("{cid}q %b", {mm.group(1)} ==? {mm.group(2)});\n')
            continue
        out.append(l)
    open(base+'_wq.sv','w').write(''.join(out))
