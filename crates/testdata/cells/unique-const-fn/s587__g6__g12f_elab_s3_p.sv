module top;
  function automatic logic [7:0] g(input int a);
    g = 8'h41;
    if (a == 1) g = 8'h42;
  endfunction
  $info("s=%s", g(2));
  initial begin #1 $finish; end
endmodule
