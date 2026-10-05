`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [63:0] v; int m;
  initial begin
    v = 64'h00000000_FFFFFFFF; case (v) inside 32'shFFFFFFFF: m = 3; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
