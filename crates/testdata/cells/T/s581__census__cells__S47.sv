module top;
  function automatic integer f(input integer x); f = x + 1; endfunction
  generate
    case (f(2))
      {64'd0, 1'bx}: begin : g_a initial $display("S47 a"); end
      3: begin : g_b initial $display("S47 b"); end
      default: begin : g_def initial $display("S47 def"); end
    endcase
  endgenerate
endmodule
