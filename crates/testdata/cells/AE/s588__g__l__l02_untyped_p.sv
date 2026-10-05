module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    fx = t;
  endfunction
  parameter P = fx(2);
  initial begin #1 $display("P=%b b=%0d", P, $bits(P)); $finish; end
  initial #100 $finish;
endmodule
