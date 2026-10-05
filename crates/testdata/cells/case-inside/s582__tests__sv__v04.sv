`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic signed [3:0] v; int m;
  logic signed [3:0] vals [0:7] = '{-4'sd2, -4'sd1, 4'sd0, 4'sd1, 4'sd2, 4'sd3, -4'sd8, 4'sd7};
  initial begin
    for (int i = 0; i < 8; i++) begin
      v = vals[i]; m = 9;
      case (v) inside [-4'sd2:4'sd1]: m = 1; [4'sd3:4'sd5]: m = 2; default: m = 0; endcase
      $display("v=%0d m=%0d", v, m);
    end
    $finish;
  end
endmodule
