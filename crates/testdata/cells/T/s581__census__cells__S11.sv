module top;
  generate
    case (4'sb1111)
      8'd15: begin : g_a initial $display("S11 a"); end
      default: begin : g_def initial $display("S11 def"); end
    endcase
  endgenerate
endmodule
