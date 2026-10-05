package q;
  function automatic logic [N:0] h(input int x); return x; endfunction
endpackage
module top;
  localparam int N = 3;
  localparam int P = q::h(1000);
  int v;
  initial begin v = q::h(1000); #1 $display("P=%0d v=%0d", P, v); $finish; end
endmodule
