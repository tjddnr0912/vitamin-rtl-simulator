module top;
  typedef enum logic signed [3:0] {M = -1, Z = 0} e_t;
  generate
    case (4'b1111)
      M: begin : g_a initial $display("E06 a"); end
      default: begin : g_def initial $display("E06 def"); end
    endcase
  endgenerate
endmodule
