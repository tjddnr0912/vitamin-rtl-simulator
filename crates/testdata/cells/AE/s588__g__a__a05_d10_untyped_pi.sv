module top;
  function automatic integer fi(input int a);
    if (a == 1) fi = 10;
  endfunction
  parameter PI = fi(2);
  initial begin #1 $display("PI=%0d bi=%0d", PI, $bits(PI)); $finish; end
  initial #100 $finish;
endmodule
