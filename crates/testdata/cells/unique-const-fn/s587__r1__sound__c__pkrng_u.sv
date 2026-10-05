package r;
  function automatic int f(input int a);
    unique if (a == 1) return 1;
    return 7;
  endfunction
endpackage
package q;
  parameter logic [r::f(2):0] P = 0;
endpackage
module top;
  initial begin $display("b=%0d", $bits(q::P)); #1 $finish; end
endmodule
