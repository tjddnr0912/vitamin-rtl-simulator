module top;
  localparam logic signed [3:0] PS = -1;
  generate
    case (4'd15)
      PS: begin : g_a initial $display("L24 a"); end
      default: begin : g_def initial $display("L24 def"); end
    endcase
  endgenerate
endmodule
