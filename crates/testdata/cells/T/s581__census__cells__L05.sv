module top;
  localparam S1 = "ab";
  generate
    case (16'h6162)
      S1: begin : g_a initial $display("L05 a"); end
      default: begin : g_def initial $display("L05 def"); end
    endcase
  endgenerate
endmodule
