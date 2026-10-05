`timescale 1ns/1ns
module t;
  logic [3:0] v; localparam logic [3:0] P = 4'b1100;
  initial begin
    v=4'b1101; $display("E41 %b", v inside {P | 4'b000x});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
