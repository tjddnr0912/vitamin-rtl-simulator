module top;
  localparam logic [3:0] PX = 4'b1x00;
  generate
    case (4'b1100)
      PX: begin : g_a initial $display("X15 a"); end
      default: begin : g_def initial $display("X15 def"); end
    endcase
  endgenerate
endmodule
