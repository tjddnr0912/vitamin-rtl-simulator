module uL;
  int i; integer ig; byte b; shortint sh; logic signed [7:0] arr [0:3]; logic signed [7:0] sp;
  typedef struct packed { logic signed [3:0] a; logic [3:0] b; } st_t; st_t st;
  localparam signed [7:0] P = -8'sd128;
  if (1) begin : g2 logic signed [7:0] s8; initial s8 = -128; end
  initial begin i = -6; ig = -6; b = -128; sh = -7; arr[1] = -128; st = 8'b1110_0000; sp = -128; end
endmodule
module t;
  uL uL();
  if (1) begin : g logic signed [7:0] s8; initial s8 = -128; end
  initial begin
    #1;
`ifdef R01
    $display("R01 %b", ((uL.i >>> 1) ==? 32'shFFFF_FFF?));
`endif
`ifdef R02
    $display("R02 %b", ((uL.ig >>> 1) ==? 32'shFFFF_FFF?));
`endif
`ifdef R03
    $display("R03 %b", ((uL.b >>> 1) ==? 8'sb1100_000?));
`endif
`ifdef R04
    $display("R04 %b", ((uL.sh >>> 1) ==? 16'sb1111_1111_1111_110?));
`endif
`ifdef R05
    $display("R05 %b", ((uL.arr[1] >>> 1) ==? 8'sb1100_000?));
`endif
`ifdef R06
    $display("R06 %b", ((uL.st.a >>> 1) ==? 4'sb111?));
`endif
`ifdef R07
    $display("R07 %b", ((uL.P >>> 1) ==? 8'sb1100_000?));
`endif
`ifdef R08
    $display("R08 %b", ((uL.st >>> 1) ==? 8'sb1111_000?));
`endif
`ifdef R09
    $display("R09 %b", ((uL.sp[7:0] >>> 1) ==? 8'sb1100_000?));
`endif
`ifdef R10
    $display("R10 %b", ((g.s8 >>> 1) ==? 8'sb1100_000?));
`endif
`ifdef R11
    $display("R11 %b", ((uL.g2.s8 >>> 1) ==? 8'sb1100_000?));
`endif
    #1 $finish;
  end
endmodule
