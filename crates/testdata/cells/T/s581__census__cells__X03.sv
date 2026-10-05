module top;
  generate
    case (1)
      4'bz: begin : g_a initial $display("X03 a"); end
      default: begin : g_def initial $display("X03 def"); end
    endcase
  endgenerate
endmodule
