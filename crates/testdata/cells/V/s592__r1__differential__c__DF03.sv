module top;
  if (f(1) == 2) begin : a initial $display("@inner"); end
  else begin : b initial $display("@else"); end
  function automatic integer f(input integer x); f = x + 1; endfunction
  initial #10 $finish;
endmodule
