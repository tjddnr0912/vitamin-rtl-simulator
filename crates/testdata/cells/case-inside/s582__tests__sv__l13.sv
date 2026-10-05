`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  int m;
  function int f(input int x); return x; endfunction
  initial begin
    case (f(-1)) inside [-4:4]: m = 1; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
