package q;
  function automatic int f(input int a);
    f = 3;
    if (a == 1) f = 10;
  endfunction
endpackage
package q2;
  import q::*;
  function automatic int g(input int a);
    return f(a) + 1;
  endfunction
endpackage
module top;
  localparam int P = q2::g(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
