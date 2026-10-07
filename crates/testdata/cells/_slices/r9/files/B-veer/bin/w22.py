# W22 (harness): the tb ties jtag_trst_n with `assign jtag_trst_n = 1'b0;`; the tap's `@(negedge trst)` misses that time-0 edge in both 4-state tools -> give it a real edge at t=2
def w22(s):
    a="  assign jtag_trst_n = 1'b0;"
    assert a in s
    return s.replace(a,"  logic cen_trst_n = 1'b1; initial #2 cen_trst_n = 1'b0; assign jtag_trst_n = cen_trst_n; // CENSUS W22",1)
