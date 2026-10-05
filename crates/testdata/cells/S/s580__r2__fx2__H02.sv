`timescale 1ns/1ns
module t;
  localparam signed [7:0] S = 8'sd84;
  localparam signed [7:0] S2 = 8'sd12;
  localparam logic [3:0] PV = 4'b1100;
  localparam Q1 = (4'd15 + 4'd1) ==? 5'b1?000;
  localparam Q2 = (~4'b0000) ==? 5'b1111?;
  localparam Q3 = (4'd15 + 4'd1) ==? 5'b0?000;
  localparam Q4 = S ==? 4'sb?100;
  localparam Q5 = S2 ==? 4'sb1?00;
  localparam Q6 = (PV ==? 4'b0?00) || (PV ==? 4'b1?00);
  localparam Q7 = (4'd15 + 4'd1) !=? 8'b0001_?000;
  localparam I1 = S inside {4'sb?100};
  localparam I2 = PV inside {4'b0?00, 4'b1?00};
  logic [((4'd15 + 4'd1) ==? 5'b1?000) : 0] rbq;
  if ((4'd15 + 4'd1) ==? 5'b1?000) begin : gq initial #1 $display("gen-q then"); end
  else begin : gqe initial #1 $display("gen-q else"); end
  initial begin
    #2 $display("Q %b %b %b %b %b %b %b I %b %b bits %0d", Q1, Q2, Q3, Q4, Q5, Q6, Q7, I1, I2, $bits(rbq));
    $finish;
  end
endmodule
