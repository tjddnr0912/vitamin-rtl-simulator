`ifdef IV
 `define IN(a,b)  ((a) ==? b)
`else
 `define IN(a,b)  ((a) inside {b})
`endif
module sub2 #(parameter int W = 1) (); initial #3 $display("OV %0d", W); endmodule
module t;
  localparam signed [3:0] PS = -4;
  sub2 #(.W(`IN(4'd15 + 4'd1, 8'b0000_?000) ? 8 : 2)) u30();
  logic [3:0] a4;
  function logic signed [3:0] fss(input logic signed [3:0] x); fss = x; endfunction
  logic signed [3:0] s4;
  for (genvar i = -4; i < -3; i++) begin : gg
    logic signed [3:0] gs;
    initial #2 $display("G30 %b", `IN(i, 4'sb1?00));
  end
  initial #1000 $finish;
  initial begin
    a4 = 4; s4 = -4;
    #1;
    $display("R30 %b", `IN(-(a4), 8'b1111_1?00));
    $display("R31 %b", `IN(a4 - 4'd5, 8'b1111_111?));
    $display("R32 %b", `IN(PS, 8'sb1111?100));
    $display("R33 %b", `IN(fss(s4), 8'sb1111?100));
  end
endmodule
