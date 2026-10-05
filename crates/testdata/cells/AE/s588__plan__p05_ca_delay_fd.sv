module top;
  function automatic logic [3:0] fd(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  function automatic logic [3:0] fx1(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    t[0] = 1'b1;
    fx1 = t;
  endfunction
  wire w;
  assign #(fd(2)) w = 1'b1;
  initial begin #1 $display("t1 w=%b", w); #1 $display("t2 w=%b", w); #1 $display("t3 w=%b", w); end
  initial #100 $finish;
endmodule
