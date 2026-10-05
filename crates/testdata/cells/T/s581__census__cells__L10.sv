module top;
  function automatic integer f(input integer x); f = x + 1; endfunction
  generate
    case (3)
      f(2): begin : g_a initial $display("L10 a"); end
      default: begin : g_def initial $display("L10 def"); end
    endcase
  endgenerate
endmodule
