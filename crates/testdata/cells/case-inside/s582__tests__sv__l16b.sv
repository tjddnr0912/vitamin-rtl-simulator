`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic signed [7:0] v; int m;
  initial begin
    v = 8'shFC; case (v) inside 4'sb1?00: m = 1; 4'b1?11: m = 2; default: m = 0; endcase $display("m=%0d", m);
    $finish;
  end
endmodule
