`timescale 1ns/1ns
module t;
  localparam logic [7:0] U = 8'h0C;
  localparam L = (U ==? 'b1?00);
  initial $display("L=%b", L);
  initial #5 $finish;
endmodule
