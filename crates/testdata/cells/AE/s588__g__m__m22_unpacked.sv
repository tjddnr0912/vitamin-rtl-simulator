module top;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  logic [7:0] m [fd(2)];
  initial begin #2 $display("s=%0d", $size(m)); $finish; end
  initial #100 $finish;
endmodule
