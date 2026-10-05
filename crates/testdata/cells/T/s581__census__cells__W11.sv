module top;
  localparam logic [3:0] P4 = 4'd15;
  generate
    case (P4 + 4'd1)
      4'd0: begin : g_a initial $display("W11 a"); end
      default: begin : g_def initial $display("W11 def"); end
    endcase
  endgenerate
endmodule
