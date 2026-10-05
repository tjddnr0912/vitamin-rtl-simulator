`timescale 1ns/1ns
module t;
  int iv;
  initial begin
    iv=12; $display("E75 %b", iv inside {4'b1?00});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
