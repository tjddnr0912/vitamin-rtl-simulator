`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  localparam L = X + {N{1'b0}};
  localparam M = (X + {N{1'b0}}) == 8'hFC;
  localparam K = (X + {N{1'b0}}) > 8'd100;
  logic [((X + {N{1'b0}}) == 8'hFC) + 3:0] v;
  initial begin $display("R: L=%0d M=%0d K=%0d vb=%0d", L, M, K, $bits(v)); #1 $finish; end
endmodule
