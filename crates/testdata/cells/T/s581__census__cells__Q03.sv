module top;
  localparam logic [15:0] P = "c";
  generate
    case (P)
      "ab": begin : g_a initial $display("Q03 a"); end
      "c": begin : g_c initial $display("Q03 c"); end
      default: begin : g_def initial $display("Q03 def"); end
    endcase
  endgenerate
endmodule
