module top;
  function automatic int f(input int a);
    f = 7;
    if (a == 1) f = 10;
  endfunction
  logic [f(2)-6:0][3:0] m2 = 8'hc3;
  logic [1:0][f(2)-4:0] m3 = 8'h3c;
  initial begin #1 $display("b2=%0d b3=%0d m2=%h m3=%h e=%h", $bits(m2), $bits(m3), m2[1], m3[0], m2); $finish; end
endmodule
