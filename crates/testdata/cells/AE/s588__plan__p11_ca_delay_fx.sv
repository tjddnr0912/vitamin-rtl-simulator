module top;
  function automatic logic [3:0] fx1(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    t[0] = 1'b1;
    fx1 = t;
  endfunction
  function automatic logic [3:0] fx(input int a);
    logic [3:0] t;
    if (a == 1) t = 4'd5;
    fx = t;
  endfunction
  reg r; wire w;
  assign #(fx(2)) w = r;
  initial begin r = 1'b0; #5 r = 1'b1; end
  initial begin #5 $strobe("t5 w=%b", w); #1 $display("t6 w=%b", w); end
  initial #100 $finish;
endmodule
