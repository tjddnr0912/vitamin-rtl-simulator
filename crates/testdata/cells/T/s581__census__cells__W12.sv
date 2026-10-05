module top;
  function automatic integer f(input integer x); f = x; endfunction
  generate
    case (4'd15 + 4'd1)
      4'd0: begin : g_a initial $display("W12 a"); end
      f(1): begin : g_b initial $display("W12 b"); end
      default: begin : g_def initial $display("W12 def"); end
    endcase
  endgenerate
endmodule
