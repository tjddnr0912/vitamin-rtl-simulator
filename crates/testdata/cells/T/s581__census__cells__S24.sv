module top;
  generate
    case (4'd15)
      8'sb11111111: begin : g_a initial $display("S24 a"); end
      default: begin : g_def initial $display("S24 def"); end
    endcase
  endgenerate
endmodule
