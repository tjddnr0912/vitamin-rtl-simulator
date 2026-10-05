module top;
  generate
    case (1)
      65'h1_0000_0000_0000_0001: begin : g_a initial $display("L19 a"); end
      default: begin : g_def initial $display("L19 def"); end
    endcase
  endgenerate
endmodule
