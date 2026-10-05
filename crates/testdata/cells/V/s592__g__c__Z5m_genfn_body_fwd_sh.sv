module top;
  localparam K = 4;
  if (1) begin : gb
    case (8'd4)
      K: begin : g function automatic integer f(); f = K; endfunction initial #1 $display("@k f=%0d", f()); end
      default: begin : g initial #1 $display("@def"); end
    endcase
    localparam K = 8;
  end
  initial #5 $finish;
endmodule
