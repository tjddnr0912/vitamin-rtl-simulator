function automatic integer f(input integer x); f = x + 100; endfunction
module top;
  generate
    if (f(1) == 2) begin : a initial $display("@region"); end
    else begin : b initial $display("@unit"); end
    function automatic integer f(input integer x); f = x + 1; endfunction
  endgenerate
  initial #10 $finish;
endmodule
