module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    fx = t;
  endfunction
  logic v [fx(2):0];
  initial begin #1 $display("s=%0d", $size(v)); $finish; end
  initial #100 $finish;
endmodule
