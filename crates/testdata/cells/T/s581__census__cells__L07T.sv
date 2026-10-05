module top;
  localparam S1 = "ab";
  generate
    case (16'h6164)
      S1 + 1: begin : g_a initial $display("L07T a"); end
      default: begin : g_def initial $display("L07T def"); end
    endcase
  endgenerate
endmodule
