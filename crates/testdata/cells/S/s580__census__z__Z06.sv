`timescale 1ns/1ns
module t;
  logic [3:0] v; logic [3:0] arr [4];
  initial begin
    for (int i = 0; i < 4; i++) arr[i] = 4'b1000 + i;
    v = 4'b1100;
    foreach (arr[i]) if (arr[i] inside {4'b10?0}) $display("Z06 %0d", i);
    #1 $finish;
  end
  initial #100 $finish;
endmodule
