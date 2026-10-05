`timescale 1ns/1ns
module t;
  logic [3:0] arr [2];
  initial begin
    arr[1]=4'b1000; $display("E73 %b", arr[1] inside {4'b1?00});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
