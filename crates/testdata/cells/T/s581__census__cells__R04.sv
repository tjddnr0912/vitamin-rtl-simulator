module top;
  localparam M = -1;
  generate
    case (M)
      32'hFFFFFFFF: begin : g_a initial $display("R04 a"); end
      default: begin : g_def initial $display("R04 def"); end
    endcase
  endgenerate
endmodule
