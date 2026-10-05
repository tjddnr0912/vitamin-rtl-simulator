`ifdef IV
 `define IN(a,b)  ((a) ==? b)
`else
 `define IN(a,b)  ((a) inside {b})
`endif
module uL;
  logic [7:0] u8; logic signed [7:0] s8; logic [1:0] k;
  function automatic logic signed [7:0] fs8(); return -8'sd128; endfunction
  initial begin u8 = 8'b1100_0000; s8 = -128; k = 1; end
endmodule
module t;
  uL uL();
  logic signed [3:0] s4; logic signed [7:0] s8; logic signed [7:0] mem [0:3];
  initial begin
    s4 = -4; s8 = -128; mem[0] = 0; mem[1] = -128; mem[2] = 0; mem[3] = 0;
    #1;
`ifdef H1
    $display("H1 %b", (t.uL.u8 ==? 8'b1?00_0000));
`endif
`ifdef H2
    $display("H2 %b", (((t.uL.k != 0) ? s4 : s4) ==? 4'sb1?00));
`endif
`ifdef H2U
    $display("H2U %b", (((t.uL.k != 0) ? s4 : s4) ==? 4'b1?00));
`endif
`ifdef H2I
    $display("H2I %b", `IN((t.uL.k != 0) ? s4 : s4, 4'sb1?00));
`endif
`ifdef H3
    $display("H3 %b", (mem[t.uL.k] ==? 8'sb1?00_0000));
`endif
`ifdef H3U
    $display("H3U %b", (mem[t.uL.k] ==? 8'b1?00_0000));
`endif
`ifdef H4
    $display("H4 %b", (s8[t.uL.k*4 +: 4] ==? 4'sb1?00));
`endif
`ifdef H5
    $display("H5 %b", (uL.s8 ==? 8'sb1?00_0000));
`endif
`ifdef H6
    $display("H6 %b", ((uL.s8 >>> 1) ==? 8'sb1100_000?));
`endif
`ifdef H7
    $display("H7 %b", (uL.fs8() ==? 8'sb1?00_0000));
`endif
`ifdef H8
    $display("H8 %b", ((mem[t.uL.k] >>> 1) ==? 8'sb1100_000?));
`endif
    #1 $finish;
  end
endmodule
