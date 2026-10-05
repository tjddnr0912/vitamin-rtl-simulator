`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b1100; $display("E44 %b", v inside {1'b1 ? 4'b1?00 : 4'b0000});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
