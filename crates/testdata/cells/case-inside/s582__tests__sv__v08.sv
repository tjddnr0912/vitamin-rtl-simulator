`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [63:0] v; int m;
  task automatic t1(input logic [63:0] x);
    v = x;
    case (v) inside [0:3]: m = 1; 32'sd7: m = 2; default: m = 0; endcase
    $display("v=%h m=%0d", v, m);
  endtask
  initial begin
    t1(64'd2); t1(64'd7); t1(64'd5); t1(64'hFFFFFFFF_FFFFFFFF); t1(64'h1_00000002);
    $finish;
  end
endmodule
