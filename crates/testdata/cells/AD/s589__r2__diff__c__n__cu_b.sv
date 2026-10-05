module top;
  localparam int K = 232;
  function automatic int g(); return 5; endfunction
  logic [31:0] v;
  initial begin #1 v = p::f(); $display("v=%0d", v); $finish; end
  initial #100 $finish;
endmodule
