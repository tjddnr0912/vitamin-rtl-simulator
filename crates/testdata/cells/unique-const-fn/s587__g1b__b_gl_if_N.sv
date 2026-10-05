module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  if (1) begin : gb
    localparam int P = f(1);
  end
  initial begin #1 $display("P=%0d", gb.P); $finish; end
endmodule
