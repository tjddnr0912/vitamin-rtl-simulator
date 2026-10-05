module top;
  wire o;
  child u(.a(1'b0), .o(o));
  initial #0 $display("z t=%0t o=%b", $time, o);
  initial #0 #0 $display("zz t=%0t o=%b", $time, o);
  initial #1 begin $display("e t=%0t o=%b", $time, o); $finish; end
endmodule
module child(input logic a, output logic o);
  always_comb o = ~a;
endmodule
