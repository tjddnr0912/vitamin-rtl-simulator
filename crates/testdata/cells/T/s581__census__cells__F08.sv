module top;
  generate
    case (4'sb1111)
      8'sd255: begin : g_a initial $display("F08 a"); end
      default: begin : g_def initial $display("F08 def"); end
    endcase
  endgenerate
endmodule
