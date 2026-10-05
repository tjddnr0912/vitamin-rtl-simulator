module tb;
  parameter logic [Nope-1:0] X = 4'h9;
  initial begin $display("DIGEST=%0d %0d", $bits(X), X); #1 $finish; end
endmodule
