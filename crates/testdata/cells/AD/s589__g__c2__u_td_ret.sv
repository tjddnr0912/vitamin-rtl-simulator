typedef logic [3:0] T;
function automatic T h(input int x); return x; endfunction
module top;
  typedef logic [7:0] T;
  localparam int P = h(1000);
  int v;
  initial begin v = h(1000); $display("v=%0d P=%0d", v, P); #1 $finish; end
endmodule
