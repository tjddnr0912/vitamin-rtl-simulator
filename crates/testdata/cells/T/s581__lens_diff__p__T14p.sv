program prg #(parameter signed [3:0] Q = -4'sd1) ();
  case (Q)
    -1: begin : a initial $display("@prg a"); end
    15: begin : b initial $display("@prg b"); end
    default: begin : d initial $display("@prg d"); end
  endcase
endprogram
module top;
  prg p1 ();
endmodule
