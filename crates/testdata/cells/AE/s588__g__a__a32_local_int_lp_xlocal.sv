module top;
  function automatic logic [3:0] f(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd10;
    f = t;
  endfunction
  localparam int P = f(2);
  initial begin #1 $display("P=%0d", P); $finish; end
  initial #100 $finish;
endmodule
