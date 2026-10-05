module top;
  function automatic integer f(input integer x);
    f = 0;
    unique if (x == 1) f = 1;
  endfunction
  localparam integer P0 = f(0);
  initial begin $display("P0=%0d", P0); #1 $finish; end
  initial #100 $finish;
endmodule
