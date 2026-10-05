`timescale 1ns/1ns
module t;
  logic [3:0] v; typedef enum logic [3:0] {A = 4'b1x00, B = 4'b0001} e_t;
  initial begin
    v=4'b1100; $display("E30 %b", v inside {A});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
