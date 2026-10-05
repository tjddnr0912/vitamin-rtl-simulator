module top;
  localparam int PI0 = 0;
  generate
    case (PI0 - 1)
      32'hFFFFFFFF: begin : g_a initial $display("R07 a"); end
      default: begin : g_def initial $display("R07 def"); end
    endcase
  endgenerate
endmodule
