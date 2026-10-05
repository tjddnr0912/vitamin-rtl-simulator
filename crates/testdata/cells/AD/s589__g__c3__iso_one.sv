module child (input logic [3:0] p);
  initial #1 $display("%m W=%0d", W);
endmodule
module top;
  localparam int W = 7;
  logic [7:0] bus = 8'hA5;
  child u (.p(bus[3:0]));
  initial #2 $finish;
endmodule
