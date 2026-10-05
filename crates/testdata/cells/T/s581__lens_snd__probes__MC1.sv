module top;
  localparam logic signed [7:0] SP = 8'sd84;
  localparam L = (SP ==? 4'sb?100);
  localparam M = (SP ==? 4'sb0100);
  initial $display("MC1 L=%0d M=%0d", L, M);
endmodule
