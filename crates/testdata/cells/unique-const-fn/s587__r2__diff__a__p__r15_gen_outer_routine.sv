module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  if (1) begin : outer
    function automatic int h(input int a); return 5; endfunction
    if (1) begin : inner
      localparam int P = f(2);
      initial #1 $display("P=%0d", P);
    end
  end
  initial #2 $finish;
endmodule
