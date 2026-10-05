module top;
  generate
    case (1)
      2: begin : g_a initial $display("M09 a"); end
    endcase
  endgenerate
endmodule
