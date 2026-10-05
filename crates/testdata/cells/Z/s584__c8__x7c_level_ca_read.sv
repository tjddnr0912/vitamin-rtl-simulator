module top;
  wire w;
  logic x;
  wire xv;
  logic t;
  assign w = 1'b1;
  assign xv = x;
  always @(w) x = ~w;
  initial t = 1'b1;
  always @(t) $display("b2 t=%0t xv=%b x=%b", $time, xv, x);
  initial #1 begin $display("e t=%0t xv=%b x=%b", $time, xv, x); $finish; end
endmodule
