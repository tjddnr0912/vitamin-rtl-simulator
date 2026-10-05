`ifdef IV
 `define IN2(a,b,c) (((a) ==? b) || ((a) ==? c))
 `define IN(a,b) ((a) ==? b)
`else
 `define IN2(a,b,c) ((a) inside {b, c})
 `define IN(a,b) ((a) inside {b})
`endif
module t;
  logic signed [7:0] acc, q; logic signed [3:0] d; logic hit, hit3, hit4;
  always_comb begin
    hit  = `IN2(acc + d, 8'sb1111_111?, 8'sb0000_000?);
    hit3 = `IN(q >>> 2, 8'sb1111_11??);
    if (`IN(acc + d, 8'sb1111_11?0)) hit4 = 1'b1; else hit4 = 1'b0;
  end
  initial begin
    acc = 0; d = -2; q = -16;
    #1 $display("Z1 %b", hit); $display("Z3 %b", hit3); $display("Z4 %b", hit4);
    #1 $finish;
  end
endmodule
