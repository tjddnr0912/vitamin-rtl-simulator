module top;
  localparam S1 = "ab";
  generate
    case (16'h6163)
      S1 + 1: begin : g_a initial $display("L07 a"); end
      default: begin : g_def initial $display("L07 def"); end
    endcase
  endgenerate
endmodule
