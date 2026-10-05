module top;
  localparam P1 = (4'b1100 ==? 4'b1?00);
  localparam [64:0] LPA = {64'd0, P1};
  generate
    case (1)
      LPA: begin : g_a initial $display("T1 a"); end
      default: begin : g_def initial $display("T1 def"); end
    endcase
  endgenerate
endmodule
