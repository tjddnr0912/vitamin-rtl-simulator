module top;
  localparam string S1 = "ab";
  generate
    case (16'h6162)
      S1: begin : g_a initial $display("Q02 a"); end
      "ab": begin : g_b initial $display("Q02 b"); end
      default: begin : g_def initial $display("Q02 def"); end
    endcase
  endgenerate
endmodule
