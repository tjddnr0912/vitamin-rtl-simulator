module top;
  localparam integer P = 1'bx ? 1 : 2;
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
