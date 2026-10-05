module sub #(parameter [127:0] P = ~128'd0) ();
  case (1)
    (P == 128'd5): begin : g5 initial #1 $display("H2 five %m"); end
    default: begin : gd initial #1 $display("H2 def %m"); end
  endcase
endmodule
module top;
  sub #(.P(128'd5)) u();
endmodule
