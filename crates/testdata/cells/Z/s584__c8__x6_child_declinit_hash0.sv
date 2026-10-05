module top;
  wire o;
  logic t;
  child u(.o(o));
  initial begin #0 $display("z t=%0t o=%b", $time, o); #0 $display("zz t=%0t o=%b", $time, o); end
  initial #1 begin $display("e t=%0t o=%b", $time, o); $finish; end
endmodule
module child(output logic o);
  logic a = 1'b0;
  always_comb o = ~a;
endmodule
