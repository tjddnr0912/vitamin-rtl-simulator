module top;
  generate
    case (1)
      $isunknown(4'b1x00): begin : g_a initial $display("X09 a"); end
      default: begin : g_def initial $display("X09 def"); end
    endcase
  endgenerate
endmodule
