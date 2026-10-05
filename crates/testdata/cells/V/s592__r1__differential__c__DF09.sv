module top;
  function automatic integer f(input integer x); f = x + 100; endfunction
  for (genvar i = 0; i < 2; i++) begin : L
    if (f(i) == i + 1) begin : a initial $display("@L%0d inner", i); end
    else begin : b initial $display("@L%0d outer", i); end
    function automatic integer f(input integer x); f = x + 1; endfunction
  end
  initial #10 $finish;
endmodule
