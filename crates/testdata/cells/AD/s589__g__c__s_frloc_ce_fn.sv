package q;
  function automatic int f(input int a); return 3; endfunction
  function int h(input int x);
    logic [f(2):0] t;
    t = 0;
    for (int i = 0; i < x; i++) t = t + 1;
    return t;
  endfunction
endpackage
module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  localparam int P = q::h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
