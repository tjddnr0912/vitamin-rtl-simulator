package p;
  localparam int W = 3;
  function automatic logic [1:0][W:0] h(); h = '1; endfunction
endpackage
module top;
  localparam int W = 7;
  int v, b;
  initial begin v = p::h(); b = $bits(p::h()); $display("v=%0d b=%0d", v, b); end
  initial #100 $finish;
endmodule
