module top;
  generate
    case (2)
      $bits(2'b00): begin : g_a initial $display("L27 a"); end
      default: begin : g_def initial $display("L27 def"); end
    endcase
  endgenerate
endmodule
