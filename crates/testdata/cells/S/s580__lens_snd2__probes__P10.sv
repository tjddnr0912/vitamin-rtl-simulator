`timescale 1ns/1ns
module t #(parameter int PI = -4, parameter logic [7:0] PU = -8'sd4, parameter PX = -8'sd4, parameter logic signed [7:0] PS = 8'sb1111_1100);
  localparam logic [3:0] P4 = -4'sd4;
  localparam bit K1 = PI ==? 4'sb1?00;
  localparam bit K2 = PU ==? 4'sb1?00;
  localparam bit K3 = PX ==? 4'sb1?00;
  localparam bit K4 = PS ==? 4'sb1?00;
  localparam bit K5 = P4 ==? 8'sb1111_1?00;
  localparam bit K6 = PU ==? 8'sb1111_1?00;
  initial begin
    #1;
    $display("R %b %b %b %b %b %b", PI ==? 4'sb1?00, PU ==? 4'sb1?00, PX ==? 4'sb1?00, PS ==? 4'sb1?00, P4 ==? 8'sb1111_1?00, PU ==? 8'sb1111_1?00);
    $display("K %b %b %b %b %b %b", K1, K2, K3, K4, K5, K6);
`ifndef NO_INSIDE
    $display("I %b %b %b %b %b", PI inside {4'sb1?00}, PU inside {4'sb1?00}, PX inside {4'sb1?00}, PS inside {4'sb1?00}, P4 inside {8'sb1111_1?00});
`endif
    #1 $finish;
  end
endmodule
