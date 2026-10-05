`timescale 1ns/1ns
module t #(parameter int NH = 2);
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  logic [((X + {NH{1'b0}}) == 8'hFC) + 3:0] v;
  initial #1 $display("vb=%0d", $bits(v));
  initial #5 $finish;
endmodule
