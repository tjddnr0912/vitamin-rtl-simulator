module ch(output logic [1:0] o);
  logic [1:0] k = 2'd3;
  always_comb o = k;
endmodule
module top;
  wire [1:0] o;
  ch u(.o(o));
  logic t = 1'b0;
  always @(t) $display("R t=%0t o=%b", $time, o);
  initial t = 1'b1;
  initial #5 $display("E o=%b", o);
  initial #10 $finish;
endmodule
