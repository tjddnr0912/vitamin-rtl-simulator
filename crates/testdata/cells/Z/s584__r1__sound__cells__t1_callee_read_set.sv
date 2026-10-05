module top;
  logic g, y, t;
  function automatic logic f(); return g; endfunction
  always_comb begin y = f(); $display("C t=%0t g=%b", $time, g); end
  always @(t) $display("R t=%0t y=%b", $time, y);
  initial begin g = 1; t = 1; #1 g = 0; #1 $finish; end
endmodule
