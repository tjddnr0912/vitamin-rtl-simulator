package pk;
  function automatic int f(input int a);
    f = 7;
    priority if (a == 1) f = 10;
  endfunction
endpackage
module top;
  import pk::*;
  localparam int P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
