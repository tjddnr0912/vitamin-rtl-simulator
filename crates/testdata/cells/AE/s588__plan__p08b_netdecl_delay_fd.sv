module top;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  reg r;
  wire #(fd(2)) w = r;
  initial begin r = 1'b0; #5 r = 1'b1; end
  initial begin #6 $display("t6 w=%b", w); #1 $display("t7 w=%b", w); #1 $display("t8 w=%b", w); end
  initial #100 $finish;
endmodule
