module top;
  localparam time P = 'x;
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
