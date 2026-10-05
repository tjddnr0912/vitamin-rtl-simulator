module top;
  generate
    case (24'h006263)
      "bc": begin : g_a initial $display("Q05 a"); end
      default: begin : g_def initial $display("Q05 def"); end
    endcase
  endgenerate
endmodule
