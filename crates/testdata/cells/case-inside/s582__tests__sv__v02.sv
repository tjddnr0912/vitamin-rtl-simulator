`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  logic [3:0] vals [0:9] = '{4'b1000, 4'b1001, 4'b1100, 4'b1101, 4'b0001, 4'b0111, 4'b0011, 4'b0000, 4'b1010, 4'b1011};
  initial begin
    for (int i = 0; i < 10; i++) begin
      v = vals[i]; m = 9;
      case (v) inside 4'b1x0z: m = 1; 4'b0zz1: m = 2; 4'b??11: m = 3; default: m = 0; endcase
      $display("v=%b m=%0d", v, m);
    end
    $finish;
  end
endmodule
