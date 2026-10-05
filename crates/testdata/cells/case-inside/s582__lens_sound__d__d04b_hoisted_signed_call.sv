module top;
  int cnt = 0; int m;
  function automatic int f(input int x); cnt = cnt + 1; return x; endfunction
  initial begin
    case (f(-1)) inside [-2:1]: m = 1; default: m = 0; endcase
    $display("A case f(-1) inside [-2:1] m=%0d cnt=%0d", m, cnt);
    case (f(-1)) inside [-2:1], 7: m = 1; default: m = 0; endcase
    $display("A2 case f(-1) inside [-2:1],7 m=%0d cnt=%0d", m, cnt);
    #1 $finish;
  end
endmodule
