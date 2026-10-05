module sub #(parameter int P = 0);
  generate
    case (4'd3)
      P: begin : g_a initial $display("O07 a"); end
      default: begin : g_def initial $display("O07 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(3)) u();
endmodule
