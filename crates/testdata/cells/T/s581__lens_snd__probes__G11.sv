module sub(input logic [1 + (4'bx100 ==? 4'b1?00):0] p);
  initial #1 $display("G11 %0d", $bits(p));
endmodule
module top;
  logic [3:0] q = 4'hF;
  sub u(.p(q));
endmodule
