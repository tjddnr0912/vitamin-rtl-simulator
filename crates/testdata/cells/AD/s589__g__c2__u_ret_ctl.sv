function automatic int f(input int a); return 3; endfunction
function automatic logic [f(2):0] h(input int x); return x; endfunction
module top;
  localparam int P = h(1000);
  int v;
  initial begin v = h(1000); $display("v=%0d P=%0d", v, P); #1 $finish; end
endmodule
