`timescale 1ns/1ns
module top;
  reg [3:0] x, inside;
  reg [3:0] m1, m2;
  initial begin
    inside = 4'd5; x = 4'd5;
    case (x)
      inside: m1 = 1;
      default: m1 = 0;
    endcase
    case (x)
      inside, 4'd9: m2 = 1;
      default: m2 = 0;
    endcase
    $display("D m1=%0d E m2=%0d", m1, m2);
    $finish;
  end
  initial #1000 $finish;
endmodule
