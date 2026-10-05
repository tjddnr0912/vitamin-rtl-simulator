`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam L = X + {2{1'b0}};
  localparam M = (X + {2{1'b0}}) == 8'hFC;
  localparam K = (X + {2{1'b0}}) > 8'd100;
  logic [((X + {2{1'b0}}) == 8'hFC) + 3:0] v;
  initial begin $display("L=%0d M=%0d K=%0d vb=%0d", L, M, K, $bits(v)); #1 $finish; end
endmodule
