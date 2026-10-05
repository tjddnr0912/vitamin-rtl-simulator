module top;
  logic k = 1'b1;
  always_comb begin $display("C1 t=%0t k=%b", $time, k); end
  always_comb begin $display("C2 t=%0t k=%b", $time, k); end
  initial begin #1 k = 0; #1 k = 1; #1 $finish; end
endmodule
