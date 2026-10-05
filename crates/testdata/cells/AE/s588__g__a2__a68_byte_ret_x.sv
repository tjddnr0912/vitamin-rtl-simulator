module top;
  function automatic byte f(input int a);
    logic [7:0] t;
    t[3:0] = 4'b0101;
    f = t;
  endfunction
  localparam int P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
