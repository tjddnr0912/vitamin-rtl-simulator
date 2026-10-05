package p;
  localparam int W = 3;
  function automatic logic [W:0] h(); h = '1; endfunction
endpackage
module top;
  import p::*;
  if (1) begin : g
    localparam int W = 7;
    function automatic logic [W:0] h(); h = '1; endfunction
    int v, b;
    initial begin v = h(); b = $bits(h()); $display("v=%0d b=%0d", v, b); end
  end
  initial #100 $finish;
endmodule
