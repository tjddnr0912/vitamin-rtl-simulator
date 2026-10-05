package pk;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
endpackage
module top;
  import pk::*;
  if (1) begin : g
    localparam int P = f(2);
    logic [f(2):0] v;
  end
  initial begin #1 $display("P=%0d bv=%0d", g.P, $bits(g.v)); $finish; end
endmodule
