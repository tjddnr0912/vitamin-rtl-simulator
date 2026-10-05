package pa;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
endpackage
package pb;
  localparam int P = pa::f(2);
endpackage
module top;
  initial begin #1 $display("P=%0d", pb::P); $finish; end
endmodule
