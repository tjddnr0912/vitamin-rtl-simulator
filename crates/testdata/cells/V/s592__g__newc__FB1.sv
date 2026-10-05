module top;
  if (1) begin : g
    localparam integer P = K;
    case (8'd2)
      P: begin : a initial #1 $display("@p"); end
      default: begin : b initial #1 $display("@def"); end
    endcase
    localparam integer K = 2;
  end
  initial #5 $finish;
endmodule
