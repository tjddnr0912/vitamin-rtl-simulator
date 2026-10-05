module child #(parameter int W = 3) (input logic [L:0] p);
  localparam int L = W;
  initial #1 $display("%m W=%0d b=%0d p=%h", W, $bits(p), p);
endmodule
module top;
  localparam int L = 7;
  logic [7:0] bus = 8'hA5;
  child u[1:0] (.p(bus));
  initial #2 $finish;
endmodule
