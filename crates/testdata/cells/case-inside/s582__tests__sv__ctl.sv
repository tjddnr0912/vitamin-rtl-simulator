`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m, k;
  logic [3:0] vals [0:3] = '{4'b1000, 4'b0010, 4'b0110, 4'b1100};
  initial begin
    for (int i = 0; i < 4; i++) begin
      v = vals[i];
      case (v) inside 4'b1?00: m = 1; [4'd1:4'd3]: m = 2; default: m = 0; endcase
      k = v inside {4'b1?00, [4'd1:4'd3]};
      $display("v=%b m=%0d k=%0d", v, m, k);
    end
    $finish;
  end
endmodule
