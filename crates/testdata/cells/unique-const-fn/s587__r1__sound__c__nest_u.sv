module top;
  function automatic int f(input int a);
    unique if (a == 1) return 1;
    return 7;
  endfunction
  function automatic int g(input int a);
    unique if (a == 1) return 1;
    return 3;
  endfunction
  int cnt = 0;
  initial begin
    repeat (f(2)'(g(3))) cnt++;
    #(f(2)'(g(3))) $display("cnt=%0d t=%0t", cnt, $time);
    #1 $finish;
  end
endmodule
