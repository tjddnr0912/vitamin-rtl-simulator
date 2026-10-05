module sub #(parameter P = 4'd0);
  generate
    case (P)
      32'hFFFFFFFF: begin : g_a initial $display("O12 a"); end
      default: begin : g_def initial $display("O12 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(-1)) u();
endmodule
