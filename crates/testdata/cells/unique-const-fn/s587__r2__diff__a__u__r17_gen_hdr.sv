module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  if (f(2) == 7) begin : g
    function automatic int h(input int a); return 1; endfunction
    initial #1 $display("g taken");
  end
  for (genvar i = 0; i < f(2) - 5; i++) begin : blk
    function automatic int h2(input int a); return 2; endfunction
    initial #1 $display("blk %0d", i);
  end
  initial #2 $finish;
endmodule
