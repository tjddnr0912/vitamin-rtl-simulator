module top;
  logic a = 1'b0;
  logic t = 1'b0;
  wire o;
  always @(t) if ($time > 0) $display("r t=%0t o=%b", $time, o);
  child u(.a(a), .o(o));
  initial begin #1; a = 1'b1; t = 1'b1; #1 $display("e t=%0t o=%b", $time, o); $finish; end
endmodule
module child(input logic a, output logic o);
  always_comb o = ~a;
endmodule
