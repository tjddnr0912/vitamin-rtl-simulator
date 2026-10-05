package pk;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  localparam int P = f(1);
  localparam int Q = f(2);
endpackage
module top;
  initial begin #1 $display("P=%0d Q=%0d", pk::P, pk::Q); $finish; end
endmodule
