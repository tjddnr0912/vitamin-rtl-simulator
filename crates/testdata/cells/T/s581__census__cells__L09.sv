module top;
  generate
    case (1)
      $clog2(2): begin : g_a initial $display("L09 a"); end
      default: begin : g_def initial $display("L09 def"); end
    endcase
  endgenerate
endmodule
