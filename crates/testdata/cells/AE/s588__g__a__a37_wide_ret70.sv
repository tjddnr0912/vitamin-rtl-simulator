module top;
  function automatic logic [69:0] f(input int a);
    if (a == 1) f = 70'd5;
  endfunction
  localparam logic [69:0] P = f(2);
  initial begin #1 $display("P=%h", P); $finish; end
  initial #100 $finish;
endmodule
