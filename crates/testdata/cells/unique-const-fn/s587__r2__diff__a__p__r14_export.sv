package q;
  function automatic int f(input int a);
    f = 3;
    if (a == 1) f = 10;
  endfunction
endpackage
package q2;
  import q::f;
  export q::f;
endpackage
module top;
  import q2::*;
  localparam int P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
