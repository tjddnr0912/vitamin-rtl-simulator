`begin_keywords "1364-2005"
`timescale 1ns/1ns
module top;
  reg [3:0] x;
  reg [3:0] inside;
  reg [3:0] m1, m2, m3;
  initial begin
    inside = 4'b0110;
    x = 4'd3;
    case (x)
      inside[2:1]: m1 = 1;
      default: m1 = 0;
    endcase
    x = 4'd7;
    case (x)
      inside + 1: m2 = 1;
      default: m2 = 0;
    endcase
    x = 4'd6;
    case (x)
      inside & 4'hF: m3 = 1;
      default: m3 = 0;
    endcase
    $display("A m1=%0d B m2=%0d C m3=%0d", m1, m2, m3);
    $finish;
  end
  initial #1000 $finish;
endmodule
`end_keywords
