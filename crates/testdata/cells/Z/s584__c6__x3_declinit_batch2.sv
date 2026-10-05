module top;
  logic a = 1'b0;
  logic y;
  logic t;
  always_comb y = ~a;
  initial t = 1'b1;
  always @(t) $display("b2 t=%0t y=%b", $time, y);
  initial #1 begin $display("e t=%0t y=%b", $time, y); $finish; end
endmodule
