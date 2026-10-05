module top;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  logic [fd(2):0] v;
  initial begin #2 $display("b=%0d", $bits(v)); $finish; end
  initial #100 $finish;
endmodule
