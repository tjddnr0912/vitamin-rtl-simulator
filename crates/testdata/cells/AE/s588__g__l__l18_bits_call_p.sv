module top;
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    fx = t;
  endfunction
  localparam int B = $bits(fx(2));
  initial begin #1 $display("B=%0d", B); $finish; end
  initial #100 $finish;
endmodule
