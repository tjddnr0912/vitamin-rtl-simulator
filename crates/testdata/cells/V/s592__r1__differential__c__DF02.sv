module top;
  if (1) begin : g
    if (f(1) == 2) begin : a initial $display("@inner"); end
    else begin : b initial $display("@else"); end
    function automatic integer f(input integer x); f = x + 1; endfunction
  end
  initial #10 $finish;
endmodule
