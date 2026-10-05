module top;
  typedef enum {A = -1, B = 3} e_t;
  parameter e_t P = A;
  generate
    case (P)
      A: begin : g_a initial $display("E02 a"); end
      B: begin : g_b initial $display("E02 b"); end
      default: begin : g_def initial $display("E02 def"); end
    endcase
  endgenerate
endmodule
