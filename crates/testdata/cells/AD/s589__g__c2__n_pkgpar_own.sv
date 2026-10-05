package q;
  function automatic int f(input int a); return 3; endfunction
  localparam int K = f(2);
endpackage
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  initial begin #1 $display("K=%0d", q::K); $finish; end
endmodule
