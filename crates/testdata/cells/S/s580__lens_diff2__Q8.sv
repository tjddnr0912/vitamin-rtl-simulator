`ifdef IV
 `define IN(a,b) ((a) ==? b)
`else
 `define IN(a,b) ((a) inside {b})
`endif
`define SH(k, e) $display(`"k %b`", e)
module sub;
  real rr = 12.0; int i32 = -1;
endmodule
module t;
  localparam UB = 36'hF_0000_0001;
  localparam UE = 4'd15 + 4'd1;
  localparam [63:0] F64 = 64'hFFFF_FFFF_FFFF_FFFF;
  localparam K1 = `IN(UB, 32'bx1);
  localparam K2 = `IN(UB, 'bx1);
  localparam K3 = `IN(UE, 5'b1?000);
  localparam K4 = `IN(F64, 'bx1);
  localparam K5 = `IN(F64, 'b0?1);
  localparam K6 = `IN(F64 + 1'b1, 'bx0);
  localparam K7 = ((F64 + 1'b1) !=? 'bx1);
  sub u();
  initial begin
    #1;
    `SH(K1, K1); `SH(K2, K2); `SH(K3, K3); `SH(K4, K4); `SH(K5, K5); `SH(K6, K6); `SH(K7, K7);
    `SH(IU2, `IN(t.u.i32, 'b1?));
    `SH(IU3, `IN(t.u.i32, 32'hFFFF_FFF?));
    `SH(IU4, `IN(t.u.i32, 4'sb111?));
    `SH(IU5, `IN(t.u.i32, 36'hF_FFFF_FFF?));
    `SH(IU6, `IN(t.u.i32, 36'shF_FFFF_FFF?));
`ifdef RR
    `SH(RR1, `IN(t.u.rr, 4'b1?00));
`endif
`ifdef IU
    `SH(IU1, `IN(t.u.i32, 'bx1));
`endif
    #1 $finish;
  end
endmodule
