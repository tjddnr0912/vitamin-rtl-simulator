package p;
  localparam int K = 8;
  function automatic int f(); localparam int K = 5; logic [K-1:0] t; t = '1; return t; endfunction
endpackage
module top;
  localparam int K = 232;
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("v=%0d", v); $finish; end
  initial #100 $finish;
endmodule
