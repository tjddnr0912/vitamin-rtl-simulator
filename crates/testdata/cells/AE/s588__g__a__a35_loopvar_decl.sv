module top;
  function automatic logic [3:0] f(input int a);
    f = 0;
    for (integer i = 0; i < 3; i = i + 1) f = f + 1;
  endfunction
  localparam logic [3:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
