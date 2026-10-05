function automatic int uf(input int a); return 3; endfunction
function automatic logic [uf(2):0] u(input int a); return a; endfunction
package q;
  function automatic logic [u(21)+2:0] h(input int x); return x; endfunction
endpackage
module top;
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display("P=%0d v=%0d", P, v); $finish; end
  initial #50 $finish;
endmodule
