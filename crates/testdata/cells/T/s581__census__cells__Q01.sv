module top;
  localparam S1 = "ab";
  generate
    case (S1)
      "ab": begin : g_a initial $display("Q01 a"); end
      default: begin : g_def initial $display("Q01 def"); end
    endcase
  endgenerate
endmodule
