module top;
  generate
    case (4'sb1111)
      -1: begin : g_a initial $display("S08 a"); end
      default: begin : g_def initial $display("S08 def"); end
    endcase
  endgenerate
endmodule
