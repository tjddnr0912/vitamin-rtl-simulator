module top;
  function automatic integer f(input integer x); f = x + 100; endfunction
  if (1) begin : g
    if (1) begin : h
      if (f(1) == 2) begin : a initial $display("@g_f"); end
      else begin : b initial $display("@mod_f"); end
    end
    function automatic integer f(input integer x); f = x + 1; endfunction
  end
  initial #10 $finish;
endmodule
