module top;
  typedef enum logic [1:0] {A, B, C} e_t;
  parameter e_t P = C;
  generate
    case (P)
      A: begin : g_a initial $display("E01 a"); end
      B: begin : g_b initial $display("E01 b"); end
      C: begin : g_c initial $display("E01 c"); end
      default: begin : g_def initial $display("E01 def"); end
    endcase
  endgenerate
endmodule
