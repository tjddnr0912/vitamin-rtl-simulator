module top;
  if (1) begin : g
    function int f(input int x);
      int r; r = 7;
      if (x == 1) r = 1; else unique if (x == 2) r = 2;
      return r;
    endfunction
    localparam int P = f(0);
    initial $display("P=%0d", P);
  end
  initial #1 $finish;
endmodule
