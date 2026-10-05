module top;
  function automatic logic [3:0] fx(input int a);
    if (a == 1) fx = 4'd10;
  endfunction
  parameter PX = fx(2);
  initial begin #1 $display("PX=%b bx=%0d", PX, $bits(PX)); $finish; end
  initial #100 $finish;
endmodule
