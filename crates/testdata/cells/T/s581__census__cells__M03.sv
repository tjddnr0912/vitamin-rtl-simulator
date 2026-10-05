module top;
  generate
    case (1)
      default: begin : g_def0 initial $display("M03 def0"); end
      1: begin : g_a initial $display("M03 a"); end
    endcase
  endgenerate
endmodule
