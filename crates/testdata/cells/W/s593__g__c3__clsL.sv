`timescale 1ns/1ns
class C #(type T = logic [3:0]);
  localparam T X = '1;
  static function void show(); $display("X=%0d lt0=%0d", X, X < 0); endfunction
endclass
module top;
  initial C#(logic signed [3:0])::show();
  initial #100 $finish;
endmodule
