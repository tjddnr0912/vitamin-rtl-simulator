module top;
  function automatic integer f(input integer x); f = x + 100; endfunction
  if (1) begin : g
    localparam integer K = f(1);
    if (K == 2) begin : a initial $display("@inner K=%0d", K); end
    else begin : b initial $display("@outer K=%0d", K); end
    function automatic integer f(input integer x); f = x + 1; endfunction
  end
  initial #10 $finish;
endmodule
