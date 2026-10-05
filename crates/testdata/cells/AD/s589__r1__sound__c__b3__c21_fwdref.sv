package p;
  function automatic logic [W:0] h(); h = '1; endfunction
  localparam int W = 3;
endpackage
module top;
  localparam int W = 7;
  int v, b;
  initial begin v = p::h(); b = $bits(p::h()); $display("v=%0d b=%0d", v, b); end
  initial #100 $finish;
endmodule
