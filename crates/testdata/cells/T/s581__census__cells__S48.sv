module top;
  function automatic integer f(input integer x); f = x + 1; endfunction
  localparam [64:0] W = {1'b1, 64'd0};
  generate
    case (f(2))
      W: begin : g_a initial $display("S48 a"); end
      default: begin : g_def initial $display("S48 def"); end
    endcase
  endgenerate
endmodule
