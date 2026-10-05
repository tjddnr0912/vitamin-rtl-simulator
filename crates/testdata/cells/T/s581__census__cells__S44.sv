module top;
  function automatic integer f(input integer x); f = x + 1; endfunction
  generate
    case (f(2))
      $isunknown(4'bx100 ==? 4'b1?00): begin : g_a initial $display("S44 a"); end
      3: begin : g_b initial $display("S44 b"); end
      default: begin : g_def initial $display("S44 def"); end
    endcase
  endgenerate
endmodule
