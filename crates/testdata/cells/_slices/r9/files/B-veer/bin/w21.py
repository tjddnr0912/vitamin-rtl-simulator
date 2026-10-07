# W21 (harness): tie the two tb inputs upstream leaves open (z in 4-state, 0 in verilator) to 0
def w21(s):
    s2,n=re.subn(r"\.dmi_uncore_enable(\s*)\(\),", r".dmi_uncore_enable\1(1'b0), /* CENSUS W21 */", s, count=1)
    s2,m=re.subn(r"\.dmi_uncore_rdata(\s*)\(\),", r".dmi_uncore_rdata\1('0), /* CENSUS W21 */", s2, count=1)
    assert n==1 and m==1
    return s2
