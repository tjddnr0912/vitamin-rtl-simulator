`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  logic [3:0] vals [0:7] = '{4'b1000, 4'b0010, 4'b0110, 4'b1100, 4'b0000, 4'b0011, 4'b0100, 4'b0001};
  initial begin
    for (int i = 0; i < 8; i++) begin
      v = vals[i]; m = 9;
      case (v) inside
        4'b1?00: m = 1;
        [4'd1:4'd3]: m = 2;
        default: m = 0;
      endcase
      $display("v=%b m=%0d", v, m);
    end
    $finish;
  end
endmodule
