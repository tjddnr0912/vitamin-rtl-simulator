module top;
  typedef enum logic signed [3:0] {M = -1, Z = 0} e_t;
  parameter e_t P = M;
  generate
    case (P)
      4'b1111: begin : g_a initial $display("E05 a"); end
      default: begin : g_def initial $display("E05 def"); end
    endcase
  endgenerate
endmodule
