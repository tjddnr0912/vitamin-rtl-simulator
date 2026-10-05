module top;
  localparam P1 = (4'b1100 ==? 4'b1?00);
  localparam [64:0] LPA = {64'd0, P1};
  generate
    case (1)
      65'd2, LPA: begin : g_a initial $display("M08 a"); end
      default: begin : g_def initial $display("M08 def"); end
    endcase
  endgenerate
endmodule
