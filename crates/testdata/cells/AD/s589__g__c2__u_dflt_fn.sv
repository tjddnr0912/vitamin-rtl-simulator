function automatic int f(input int a); return 3; endfunction
function automatic int h(input int x, input int k = f(2)); return x + k; endfunction
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  localparam int P = h(1000);
  int v;
  initial begin v = h(1000); $display("v=%0d P=%0d", v, P); #1 $finish; end
endmodule
