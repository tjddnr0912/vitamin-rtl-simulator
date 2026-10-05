package pk;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
endpackage
module top;
  localparam int P = pk::f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
