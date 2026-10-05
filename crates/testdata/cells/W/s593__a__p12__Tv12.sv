`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T X = -4;
  localparam int N = 2;
  localparam L = X + {N{1'b0}};
  localparam M = (X + {N{1'b0}}) == 8'hFC;
  localparam K = (X + {N{1'b0}}) > 8'd100;
  logic [((X + {N{1'b0}}) == 8'hFC) + 3:0] v;
  initial begin $display("L=%0d M=%0d K=%0d vb=%0d", L, M, K, $bits(v)); end
endmodule
module t;
  m #(.T(logic signed [7:0])) u();
  initial #5 $finish;
endmodule
