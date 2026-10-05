module top;
  function automatic integer f(input integer x); f = x + 1; endfunction
  generate
    case (f(2))
      65'd3: begin : g_a initial $display("S43 a"); end
      default: begin : g_def initial $display("S43 def"); end
    endcase
  endgenerate
endmodule
