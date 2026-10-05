package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic int h(input int x, input int k = f(2)); return x + k; endfunction
endpackage
module top;
  localparam int f = 7;
  localparam int P = q::h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
