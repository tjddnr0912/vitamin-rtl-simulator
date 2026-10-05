`timescale 1ns/1ns
module t;
  localparam logic [3:0] PV = 4'b1100;
  localparam L = PV inside {4'b0?00, 4'b1?00};
  initial begin $display("L %b", L); $finish; end
endmodule
