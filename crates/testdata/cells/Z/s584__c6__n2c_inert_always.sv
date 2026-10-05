module top;
  logic a;
  logic t;
  always a = 1'b1;
  initial t = 1'b1;
  always @(t) $display("b2 t=%0t a=%b", $time, a);
  initial #1 begin $display("e t=%0t a=%b", $time, a); $finish; end
endmodule
