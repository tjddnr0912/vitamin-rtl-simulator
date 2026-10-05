module top;
  generate
    case (4)
      4'b1z00: begin : g_a initial $display("X04 a"); end
      default: begin : g_def initial $display("X04 def"); end
    endcase
  endgenerate
endmodule
