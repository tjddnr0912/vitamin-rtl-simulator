`define HRNG f(2)
package q;
  function automatic int f(input int a); return 3; endfunction
  function automatic logic [`HRNG:0] h(input int x); return x; endfunction
endpackage
module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); $display("v=%0d P=%0d", v, P); #1 $finish; end
endmodule
