module top;
  logic s, y;
  always_comb y = s;
  initial s = 1'b1;
  always @(s) $display("b2 t=%0t s=%b y=%b", $time, s, y);
  initial #1 begin $display("e t=%0t y=%b", $time, y); $finish; end
endmodule
