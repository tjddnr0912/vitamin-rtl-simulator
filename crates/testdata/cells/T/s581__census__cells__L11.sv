module top;
  typedef enum logic [1:0] {E0, E1, E2} e_t;
  generate
    case (2)
      E2: begin : g_a initial $display("L11 a"); end
      default: begin : g_def initial $display("L11 def"); end
    endcase
  endgenerate
endmodule
