module child #(parameter int W = 3) (input logic [W:0] p);
  initial #1 $display("%m W=%0d b=%0d p=%h", W, $bits(p), p);
endmodule
module top;
  localparam int W = 7;
  logic [7:0] bus = 8'hA5;
  child u[1:0] (.p(bus));
  initial #2 $finish;
endmodule
