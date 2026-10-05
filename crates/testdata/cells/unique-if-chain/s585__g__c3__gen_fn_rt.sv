module top;
  if (1) begin : g
    function int f(input int x);
      int r; r = 7;
      unique if (x == 1) r = 1; else if (x == 2) r = 2;
      return r;
    endfunction
    initial begin #1 $display("t=%0t f0=%0d", $time, f(0)); end
  end
  initial #2 $finish;
endmodule
