module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  int n1 = 0, n2 = 0;
  int x = 255;
  logic [31:0] y = 255;
  initial begin
    repeat (f(2)'(x)) n1++;
    repeat (f(2)'(y)) n2++;
    #1 $display("n1=%0d n2=%0d", n1, n2); $finish;
  end
endmodule
