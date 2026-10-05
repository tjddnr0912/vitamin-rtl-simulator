`timescale 1ns/1ns
module t;
  logic [3:0] v; localparam logic [3:0] L = 4'b1?00;
  initial begin
    v=4'b1100; $display("E21 %b", v inside {L});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
