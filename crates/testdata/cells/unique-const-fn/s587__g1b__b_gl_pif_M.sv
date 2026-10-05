module top;
  function automatic int f(input int a);
    f = 7;
    priority if (a == 1) f = 10;
  endfunction
  if (1) begin : gb
    localparam int P = f(2);
  end
  initial begin #1 $display("P=%0d", gb.P); $finish; end
endmodule
