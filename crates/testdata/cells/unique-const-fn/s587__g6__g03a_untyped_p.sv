module top;
  function automatic logic [3:0] f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  localparam P = f(2);
  initial begin #1 $display("P=%0d b=%0d", P, $bits(P)); $finish; end
endmodule
