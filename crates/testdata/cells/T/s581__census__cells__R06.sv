module top;
  localparam P0 = 0;
  generate
    case (P0 - 1)
      32'hFFFFFFFF: begin : g_a initial $display("R06 a"); end
      default: begin : g_def initial $display("R06 def"); end
    endcase
  endgenerate
endmodule
