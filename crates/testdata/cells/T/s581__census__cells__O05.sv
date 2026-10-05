module sub #(parameter P = 0);
  generate
    case (P)
      -1: begin : g_a initial $display("O05 a"); end
      default: begin : g_def initial $display("O05 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(8'hFF)) u();
endmodule
