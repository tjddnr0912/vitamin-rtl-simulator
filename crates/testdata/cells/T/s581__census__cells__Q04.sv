module top;
  generate
    case (8'h63)
      "c": begin : g_c initial $display("Q04 c"); end
      "ab": begin : g_a initial $display("Q04 a"); end
      default: begin : g_def initial $display("Q04 def"); end
    endcase
  endgenerate
endmodule
