module sub #(parameter [64:0] P = 65'h1_0000_0000_0000_0000) ();
  case (P)
    65'd7: begin : g7 initial #1 $display("H3 seven %m"); end
    default: begin : gd initial #1 $display("H3 def %m"); end
  endcase
endmodule
module top;
  sub #(.P(7)) u [1:0] ();
endmodule
