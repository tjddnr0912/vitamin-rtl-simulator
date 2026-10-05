module top;
  typedef enum bit [3:0] {A=1, B=2, C=5} e_t;
  function automatic logic [3:0] fd(input int a);
    e_t t;
    if (a == 1) t = C;
    if (t == 4'd0) fd = 4'd1; else fd = 4'd2;
  endfunction
  int a2 = 2;
  reg r; wire w;
  assign #(fd(a2)) w = r;
  initial begin r = 1'b0; #5 r = 1'b1; end
  initial begin #6 $strobe("t6 w=%b", w); #1 $strobe("t7 w=%b", w); #1 $strobe("t8 w=%b", w); end
  initial #100 $finish;
endmodule
