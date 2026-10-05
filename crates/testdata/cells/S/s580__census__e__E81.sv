`timescale 1ns/1ns
module t;
  logic [3:0] v; logic [3:0] e;
  initial begin
    v=4'b1100; e=4'b1100; $display("E81 %b", v inside {e, 4'b0?00});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
