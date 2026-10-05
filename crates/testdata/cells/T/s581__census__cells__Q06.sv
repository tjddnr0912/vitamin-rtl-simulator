module top;
  generate
    case (16'h6263)
      "\0bc": begin : g_a initial $display("Q06 a"); end
      default: begin : g_def initial $display("Q06 def"); end
    endcase
  endgenerate
endmodule
