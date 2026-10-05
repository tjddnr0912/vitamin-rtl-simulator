`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  function automatic logic [3:0] g(input int n); $display("g(%0d)", n); return n[3:0]; endfunction
  initial begin
    v = 4'd5; case (v) inside [g(3):g(6)]: m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
