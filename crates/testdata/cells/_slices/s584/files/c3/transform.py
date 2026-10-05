#!/usr/bin/env python3
"""Hand-spell candidate t0 rules in SV, for the PRE binary.
R1  : move every module-level always_comb/always_latch item after all other items of its module (same batch, after initials/always).
R2  : always_comb/latch -> `always @*` (level waiter armed at seeding, no seed run) + `always @(__t0w) BODY` with `assign __t0w = 1'b1`
      (the t0 settle wake: held to the FRONT of the second Active batch, after batch 1 propagated + CA settle). forward order.
R2r : R2 with the held copies in reverse source order.
R3  : `always @*` + `initial #0 BODY` appended at module end in REVERSE source order (iverilog: implicit run after the first #0 batch).
R3f : R3 forward order.
Only 2-space-indented module items are rewritten; a cell with a deeper always_comb (generate) is reported as SKIP."""
import re, sys, os
KW = re.compile(r"^  (always_comb|always_latch)\b(.*)$")
def items(lines, i):
    """return (end_index_exclusive) of the item starting at line i"""
    l = lines[i]
    b = len(re.findall(r"\bbegin\b", l)); e = len(re.findall(r"\bend\b", l))
    if b == e:
        return i + 1
    j = i + 1
    depth = b - e
    while j < len(lines):
        depth += len(re.findall(r"\bbegin\b", lines[j])) - len(re.findall(r"\bend\b", lines[j]))
        j += 1
        if depth == 0:
            return j
    raise SystemExit("unbalanced")
def spell(text, rule):
    if re.search(r"^ {3,}(always_comb|always_latch)\b", text, re.M):
        return None
    out = []
    lines = text.split("\n")
    i = 0
    mod = []  # lines of current module
    combs = []
    inmod = False
    def flush_mod():
        nonlocal mod, combs
        body = mod[:-1]; endl = mod[-1]
        app = []
        if rule == "R1":
            for c in combs: app += c
        elif rule in ("R2", "R2r"):
            if combs: app.append("  wire __t0w; assign __t0w = 1'b1;")
            seq = combs if rule == "R2" else list(reversed(combs))
            for c in seq:
                m = KW.match(c[0]); app += ["  always @(__t0w)" + m.group(2)] + c[1:]
        elif rule in ("R3", "R3f"):
            seq = list(reversed(combs)) if rule == "R3" else combs
            for c in seq:
                m = KW.match(c[0]); app += ["  initial #0" + m.group(2)] + c[1:]
        if rule in ("R4", "R4f", "R4t"):
            seq = list(reversed(combs)) if rule == "R4" else combs
            if rule == "R4t":
                def wr(c):
                    s = " ".join(c); s = re.sub(r'"[^"]*"', "", s)
                    return set(re.findall(r"\b([a-z_][a-z0-9_]*)\s*(?:\[[^\]]*\])?\s*=(?!=)", s))
                def rd(c):
                    s = " ".join(c[0:]); s = re.sub(r'"[^"]*"', "", s)
                    s = KW.sub("", s) if False else s
                    return set(re.findall(r"\b([a-z_][a-z0-9_]*)\b", s))
                rem = list(combs); seq = []
                while rem:
                    for c in rem:
                        if not any(o is not c and (wr(o) & (rd(c) - wr(c))) for o in rem):
                            seq.append(c); rem.remove(c); break
                    else:
                        seq += rem; rem = []
            top = []
            for c in seq:
                m = KW.match(c[0]); top += ["  initial #0" + m.group(2)] + c[1:]
            # insert right after the module header + declarations? Put right after the header line (first line) -> before any process.
            # declarations must precede use in some tools; vita accepts forward refs inside a module. Insert after the last
            # declaration line that precedes the first process/instance.
            k = 1
            while k < len(body) and not re.match(r"^  (initial|always|assign|[a-z_][a-z0-9_]* +(#\(|u\b|u\())", body[k]) :
                k += 1
            return body[:k] + top + body[k:] + [endl]
        return body + app + [endl]
    while i < len(lines):
        l = lines[i]
        if re.match(r"^module\b", l):
            inmod = True; mod = [l]; combs = []; i += 1; continue
        if inmod:
            m = KW.match(l)
            if m:
                j = items(lines, i)
                item = lines[i:j]
                combs.append(item)
                if rule != "R1":
                    mod += ["  always @*" + m.group(2)] + item[1:]
                i = j; continue
            mod.append(l)
            if re.match(r"^endmodule\b", l):
                out += flush_mod(); inmod = False
            i += 1; continue
        out.append(l); i += 1
    return "\n".join(out)
if __name__ == "__main__":
    src, dst = sys.argv[1], sys.argv[2]
    os.makedirs(dst, exist_ok=True)
    for f in sorted(os.listdir(src)):
        if not f.endswith(".sv"): continue
        t = open(os.path.join(src, f)).read()
        for r in ("R1", "R2", "R2r", "R3", "R3f", "R4", "R4f", "R4t"):
            s = spell(t, r)
            if s is None:
                print("SKIP", f); break
            os.makedirs(os.path.join(dst, r), exist_ok=True)
            open(os.path.join(dst, r, f), "w").write(s)
