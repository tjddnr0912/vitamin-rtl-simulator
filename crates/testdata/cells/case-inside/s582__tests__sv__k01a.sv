`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m; logic [3:0] arr [0:1] = '{4'd5, 4'd7};
  initial begin
    v = 4'd5; case (v) inside arr: m = 1; 4'd2: m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
