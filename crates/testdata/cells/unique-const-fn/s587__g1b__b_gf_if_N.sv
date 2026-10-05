module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  for (genvar i = 0; i < f(1); i++) begin : g
    localparam int K = i;
    if (i == 6) begin : l
      initial begin #1 $display("g6=%0d", K); $finish; end
    end
  end
endmodule
