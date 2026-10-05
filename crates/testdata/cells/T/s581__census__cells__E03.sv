module top;
  typedef enum {A = -1, B = 3} e_t;
  parameter e_t P = A;
  generate
    case (P)
      32'hFFFFFFFF: begin : g_a initial $display("E03 a"); end
      B: begin : g_b initial $display("E03 b"); end
      default: begin : g_def initial $display("E03 def"); end
    endcase
  endgenerate
endmodule
