package p; localparam signed [7:0] PS = 8'hF0; endpackage
module s #(parameter signed [3:0] S = 4'hF, parameter unsigned [3:0] U = 4'sd8)();
  initial #1 $display("A s=%0d u=%0d", S, U);
endmodule
module t; import p::*;
  localparam signed [3:0] W = 8'hF7; localparam unsigned [3:0] W2 = -1;
  s #(.S(8'hFE), .U(8'sh88)) u1();
  initial #2 $display("B ps=%0d w=%0d w2=%0d", PS, W, W2);
  initial #100 $finish;
endmodule
