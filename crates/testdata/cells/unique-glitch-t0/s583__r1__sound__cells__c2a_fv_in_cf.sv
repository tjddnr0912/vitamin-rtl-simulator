module top;
  function automatic void fv(input integer x);
    if (x == 1) ; else unique if (x == 2) ;
  endfunction
  function automatic integer cf(input integer n);
    fv(n);
    cf = n + 1;
  endfunction
  localparam integer P = cf(3);
  initial begin $display("P=%0d", P); #1 $finish; end
  initial #100 $finish;
endmodule
