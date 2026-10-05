module top;
  wire o;
  logic t;
  child u(.a(1'b0), .o(o));
  initial t = 1'b1;
  always @(t) $display("b2 t=%0t o=%b", $time, o);
  initial #1 begin $display("e t=%0t o=%b", $time, o); $finish; end
endmodule
module child(input logic a, output logic o);
  always_comb o = ~a;
endmodule
