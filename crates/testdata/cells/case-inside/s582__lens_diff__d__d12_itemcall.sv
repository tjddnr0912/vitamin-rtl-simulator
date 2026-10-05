`timescale 1ns/1ns
module top;
  int v, m, cnt;
  function automatic int g(input int n); cnt = cnt + 1; $display("g(%0d)", n); return n; endfunction
  function automatic bit [7:0] h(input int n); cnt = cnt + 1; $display("h(%0d)", n); return n[7:0]; endfunction
  initial begin
    cnt = 0; v = 1;
    case (v) inside g(1): m = 1; g(2): m = 2; default: m = 0; endcase
    $display("A m=%0d cnt=%0d", m, cnt);
    cnt = 0; v = 7;
    case (v) inside [g(3):g(9)]: m = 3; default: m = 0; endcase
    $display("B m=%0d cnt=%0d", m, cnt);
    cnt = 0;
    case (v[7:0]) inside h(7): m = 4; h(8): m = 5; default: m = 0; endcase
    $display("C m=%0d cnt=%0d", m, cnt);
    $finish;
  end
  initial #1000 $finish;
endmodule
