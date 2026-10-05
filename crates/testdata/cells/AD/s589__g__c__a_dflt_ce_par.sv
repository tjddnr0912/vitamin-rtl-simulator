package q;
  localparam int W = 3;
  function automatic int h(input int x, input int k = W); return x + k; endfunction
endpackage
module top;
  localparam int W = 7;
  localparam int P = q::h(1000);
  initial begin #1 $display("P=%0d", P); $finish; end
endmodule
