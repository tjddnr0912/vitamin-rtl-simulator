module top;
  int cnt = 0; int m;
  function automatic int f(input int x); cnt = cnt + 1; return x; endfunction
  function automatic int g(input int x, output int o); o = x; return x; endfunction
  int o;
  initial begin
    case (f(-1)) inside [-2:1]: m = 1; default: m = 0; endcase
    $display("A case f(-1) inside [-2:1] m=%0d cnt=%0d", m, cnt);
    case (g(-1, o)) inside [-2:1]: m = 1; default: m = 0; endcase
    $display("B case g(-1,o) inside [-2:1] m=%0d o=%0d", m, o);
    case (f(-1)) inside 4'sb1111: m = 1; default: m = 0; endcase
    $display("C case f(-1) inside 4'sb1111 m=%0d cnt=%0d", m, cnt);
    case (f(-1)) 4'sb1111: m = 1; default: m = 0; endcase
    $display("D plain case f(-1) 4'sb1111 m=%0d cnt=%0d", m, cnt);
    $display("E if f(-1) inside {[-2:1]} = %0d cnt=%0d", f(-1) inside {[-2:1]}, cnt);
    #1 $finish;
  end
endmodule
