module top;
  if (1) begin : gb
    typedef enum logic [3:0] {A, B, C} e_t;
    localparam P = C + 1;
    case (P)
      3: begin : g initial #1 $display("@three P=%0d", P); end
      default: begin : g initial #1 $display("@def P=%0d", P); end
    endcase
  end
  initial #5 $finish;
endmodule
