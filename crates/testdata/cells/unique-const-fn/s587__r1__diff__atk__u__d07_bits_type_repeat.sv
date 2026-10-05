module top;
  function automatic int f(input int a);
    f = 7;
    unique if (a == 1) f = 10;
  endfunction
  int n = 0, m = 0, k = 0;
  initial begin
    repeat ($bits(logic [f(2):0])) n++;
    repeat ($bits(logic [f(2):0]) + f(2)) m++;
    repeat (f(2) + $bits(logic [f(2):0])) k++;
    #1 $display("n=%0d m=%0d k=%0d", n, m, k); $finish;
  end
endmodule
