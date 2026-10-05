module top;
  function automatic integer f(input integer x);
    f = 0;
    unique if (x == 1) f = 1;
  endfunction
  localparam integer P1 = f(1);
  initial begin $display("P1=%0d", P1); #1 $finish; end
  initial #100 $finish;
endmodule
