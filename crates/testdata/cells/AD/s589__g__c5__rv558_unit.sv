localparam int W = 4;
function automatic logic [W-1:0] g(input int a); g = '1; endfunction
module top;
  localparam int W = 8;
  localparam int P = g(0);
  int v;
  initial begin v = g(0); #1 $display("P=%0d v=%0d", P, v); $finish; end
endmodule
