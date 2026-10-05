package p;
  localparam int W = 3;
  function automatic logic [W:0] h(); h = '1; endfunction
endpackage
localparam int W = 7;
function automatic logic [W:0] h(); h = '1; endfunction
module top;
  int v, b;
  initial begin v = h(); b = $bits(h()); $display("v=%0d b=%0d", v, b); end
  initial #100 $finish;
endmodule
