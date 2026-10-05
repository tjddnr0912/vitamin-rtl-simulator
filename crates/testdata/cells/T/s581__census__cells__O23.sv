module sub #(parameter P = 0);
  generate
    case (P)
      65'h1_0000_0000_0000_0001: begin : g_a initial $display("O23 a"); end
      default: begin : g_def initial $display("O23 def"); end
    endcase
  endgenerate
endmodule
module top;
  sub #(.P(65'h1_0000_0000_0000_0001)) u();
endmodule
