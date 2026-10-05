module top;
  localparam integer K = 1;
  if (1) begin : g
    if (K == 2) begin : a
      function automatic integer f(input integer x); f = x + 20; endfunction
    end else begin : b
      function automatic integer f(input integer x); f = x + 10; endfunction
    end
    localparam integer K = 2;
  end
  initial #1 $display("@f=%0d", g.b.f(1));
  initial #10 $finish;
endmodule
