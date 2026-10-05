`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  int m;
  function automatic string sf(input int n); $display("sf(%0d)", n); return "b"; endfunction
  initial begin
    case (sf(1)) inside "b": m = 3; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
