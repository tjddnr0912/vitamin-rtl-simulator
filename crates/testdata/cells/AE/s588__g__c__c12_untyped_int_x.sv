module top;
  parameter P = 'x;
  initial begin #1 $display("P=%b b=%0d", P, $bits(P)); $finish; end
  initial #100 $finish;
endmodule
