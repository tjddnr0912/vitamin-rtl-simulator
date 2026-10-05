package r;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
endpackage
package q;
  localparam logic [r::f(2):0] K = '1;
endpackage
module top;
  initial begin #1 $display("b=%0d", $bits(q::K)); $finish; end
endmodule
