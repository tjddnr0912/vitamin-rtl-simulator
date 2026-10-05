`timescale 1ns/1ns
module t;
  logic [3:0] v; localparam L = 4'b1100;
  initial begin
    v=4'b1100; $display("E25 %b", v inside {L});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
