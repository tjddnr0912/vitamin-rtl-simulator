`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  logic [3:0] vals [0:8] = '{4'b1x00, 4'bx100, 4'b011x, 4'b100z, 4'bxxxx, 4'b1z00, 4'b1001, 4'bzzzz, 4'b10x1};
  initial begin
    for (int i = 0; i < 9; i++) begin
      v = vals[i]; m = 9;
      case (v) inside 4'b1?00: m = 1; 4'b0110: m = 2; [4'd8:4'd9]: m = 3; default: m = 0; endcase
      $display("v=%b m=%0d", v, m);
    end
    $finish;
  end
endmodule
