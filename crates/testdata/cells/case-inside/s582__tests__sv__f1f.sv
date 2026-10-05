`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  reg [3:0] x, inside; reg [3:0] m1;
  initial begin
    inside = 4'd5; x = 4'd1;
    case (x) inside == 4'd5: m1 = 1; default: m1 = 0; endcase
    $display("H m1=%0d", m1);
    $finish;
  end
endmodule
