module sub #(parameter [64:0] P = 0);
  generate
    case (P)
      65'd1: begin : g_a initial $display("O06 a"); end
      default: begin : g_def initial $display("O06 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(1)) u();
endmodule
