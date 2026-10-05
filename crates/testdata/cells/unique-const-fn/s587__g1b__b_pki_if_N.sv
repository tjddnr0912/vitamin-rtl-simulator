package pk;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
endpackage
module top;
  import pk::*;
  localparam int P = f(1);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
