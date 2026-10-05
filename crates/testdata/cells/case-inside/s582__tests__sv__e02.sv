`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  int m;
  function automatic logic [3:0] g(input int n); $display("g(%0d)", n); return n[3:0]; endfunction
  function automatic int fn(input int n);
    case (g(n)) inside 4'd1: fn = 1; [4'd5:4'd6]: fn = 2; 4'b1?00: fn = 3; 4'd3: fn = 4; default: fn = 0; endcase
  endfunction
  task automatic tk(input int n, output int o);
    case (g(n)) inside 4'd1: o = 1; [4'd5:4'd6]: o = 2; 4'b1?00: o = 3; 4'd3: o = 4; default: o = 0; endcase
  endtask
  initial begin
    case (g(3)) inside 4'd1: m = 1; [4'd5:4'd6]: m = 2; 4'b1?00: m = 3; 4'd3: m = 4; default: m = 0; endcase
    $display("mod m=%0d", m);
    case (g(7)) inside default: m = 5; endcase
    $display("defonly m=%0d", m);
    m = fn(3); $display("fn m=%0d", m);
    tk(3, m); $display("tk m=%0d", m);
    $finish;
  end
endmodule
