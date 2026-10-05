`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] v; int m; bit [3:0] k2; int ki;
  initial begin
    k2 = 4'd3; ki = 5;
    for (int i = 0; i < 9; i++) begin
      v = i[3:0];
      case (v) inside k2: m = 1; ki: m = 2; default: m = 0; endcase
      $display("var v=%0d m=%0d", v, m);
    end
    for (int i = 6; i < 11; i++) begin
      v = i[3:0];
      case (v) inside [k2 + 4'd5 : 4'd9]: m = 3; default: m = 0; endcase
      $display("rng v=%0d m=%0d", v, m);
    end
    $finish;
  end
endmodule
