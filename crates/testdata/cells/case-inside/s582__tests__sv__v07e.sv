`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m;
  logic [3:0] vals [0:2] = '{4'd1, 4'd2, 4'd3};
  initial begin
    for (int i = 0; i < 3; i++) begin
      v = vals[i]; m = 9;
      case (v) inside 4'd1: ; 4'd2: m = 2; default: m = 0; endcase
      $display("empty v=%0d m=%0d", v, m);
    end
    $finish;
  end
endmodule
