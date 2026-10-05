module top;
  typedef enum logic [1:0] {A, B, C} e_t;
  generate
    case (2'b10)
      C: begin : g_c initial $display("E04 c"); end
      A: begin : g_a initial $display("E04 a"); end
      default: begin : g_def initial $display("E04 def"); end
    endcase
  endgenerate
endmodule
