`timescale 1ns/1ns
module m #(parameter logic signed [7:0] X = 0, parameter int N = 1);
  localparam L = ((X + {N{1'b0}}) == 8'hFC);
  localparam V = X + {N{1'b0}};
  logic [((X + {N{1'b0}}) == 8'hFC) + 3:0] v;
  initial #1 $display("L=%0d V=%0d B=%0d vb=%0d", L, V, $bits(V), $bits(v));
endmodule
module t;
  m #(.X(-4), .N(2)) u();
  initial #5 $finish;
endmodule
