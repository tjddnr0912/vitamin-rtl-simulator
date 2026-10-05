module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  if (1) begin : g
    $info("el=%0d", f(2));
  end
  initial begin #1 $finish; end
endmodule
