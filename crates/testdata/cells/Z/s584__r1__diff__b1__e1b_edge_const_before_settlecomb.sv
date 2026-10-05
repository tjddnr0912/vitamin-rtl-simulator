// posedge settle wake declared before a settle-woken comb (comb reads x) it reads
module top;
  wire x = 1'b1;
  logic [1:0] c = 2'd2;
  logic [1:0] d;
  always @(posedge x) $display("P t=%0t d=%b", $time, d);
  always_comb d = c & {2{x}};
  initial $display("I t=%0t d=%b", $time, d);
  initial #5 $display("E t=%0t d=%b", $time, d);
  initial #10 $finish;
endmodule
