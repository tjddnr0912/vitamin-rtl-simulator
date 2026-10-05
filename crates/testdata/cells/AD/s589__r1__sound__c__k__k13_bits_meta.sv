package p;
  localparam logic [4:0] W = 5'd2;
  function automatic logic [$bits(W)-1:0] h(); h = '1; endfunction
endpackage
module top;
  localparam logic [7:0] W = 8'd2;
  int v, b;
  initial begin v = p::h(); b = $bits(p::h()); $display("v=%0d b=%0d", v, b); end
  initial #100 $finish;
endmodule
