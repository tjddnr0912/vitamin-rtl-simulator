module top;
  localparam integer A = 5;
  if (1) begin : g
    case (1)
      A: begin : a initial $display("@enumA"); end
      default: begin : d initial $display("@def"); end
    endcase
    typedef enum integer {Z = 0, A = 1} e_t;
  end
  initial #10 $finish;
endmodule
