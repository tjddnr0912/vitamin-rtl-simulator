module top;
  function automatic logic [7:0] g(input int a);
    g = "A";
    unique if (a == 1) g = "B";
  endfunction
  $info("s=%s", g(2));
  initial begin #1 $finish; end
endmodule
