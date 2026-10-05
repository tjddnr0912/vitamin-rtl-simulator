module top;
  generate
    case (8'sd255)
      -1: begin : g_a initial $display("F09 a"); end
      default: begin : g_def initial $display("F09 def"); end
    endcase
  endgenerate
endmodule
