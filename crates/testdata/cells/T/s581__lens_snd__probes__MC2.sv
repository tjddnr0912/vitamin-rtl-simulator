module top;
  localparam logic signed [3:0] SN = -4;
  localparam L = (SN ==? 8'sb1111_?100);
  localparam M = (SN ==? 8'b1111_?100);
  initial $display("MC2 L=%0d M=%0d", L, M);
endmodule
