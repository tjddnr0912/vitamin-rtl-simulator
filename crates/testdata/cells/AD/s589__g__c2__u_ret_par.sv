localparam int W = 3;
function automatic logic [W:0] h(input int x); return x; endfunction
module top;
  localparam int W = 7;
  localparam int P = h(1000);
  int v;
  initial begin v = h(1000); $display("v=%0d P=%0d", v, P); #1 $finish; end
endmodule
