`timescale 1ns/1ns
module t;
  logic [3:0] v; logic [3:0] arr [2];
  initial begin
    arr[0]=4'b0000; arr[1]=4'b1100; v=4'b1100; $display("E70 %b", v inside {arr});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
