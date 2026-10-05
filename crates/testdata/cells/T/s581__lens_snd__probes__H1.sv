module sub #(parameter [127:0] P = ~128'd0) ();
  case (P)
    128'd5: begin : g5 initial #1 $display("H1 five %m"); end
    default: begin : gd initial #1 $display("H1 def %m"); end
  endcase
endmodule
module top;
  sub #(.P(5)) u();
endmodule
