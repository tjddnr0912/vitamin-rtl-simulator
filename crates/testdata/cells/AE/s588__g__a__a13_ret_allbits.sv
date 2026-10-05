module top;
  function automatic logic [7:0] f(input int a);
    integer i;
    for (i = 0; i < 4; i = i + 1) f[i*2 +: 2] = i;
  endfunction
  localparam logic [7:0] P = f(2);
  initial begin #1 $display("P=%b", P); $finish; end
  initial #100 $finish;
endmodule
