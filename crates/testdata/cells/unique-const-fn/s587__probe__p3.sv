module top;
  function automatic int fu(input int a);
    fu = 7;
    unique case (a) 1: fu = 10; 3: fu = 30; endcase
  endfunction
  function automatic int fp(input int a);
    fp = 7;
    priority case (a) 1: fp = 10; 3: fp = 30; endcase
  endfunction
  function automatic int fi(input int a);
    fi = 7;
    unique if (a == 1) fi = 10;
  endfunction
  int x, y, z, a;
  initial begin a = 2; #1 x = fu(a); #1 y = fp(a); #1 $display("x=%0d y=%0d", x, y); $finish; end
endmodule
