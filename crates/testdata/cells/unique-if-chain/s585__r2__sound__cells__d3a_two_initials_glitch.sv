module top;
  logic a, b; logic [1:0] r;
  always @(a or b) begin r = 0; unique if (a) r = 1; else if (b) r = 2; end
  initial a = 0;
  initial b = 1;
  initial begin #1 $display("t=%0t r=%0d", $time, r); #1 b = 0; #1 $display("t=%0t r=%0d", $time, r); #1 $finish; end
endmodule
