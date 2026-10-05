`timescale 1ns/1ns
module t;
  logic signed [3:0] s4;
  logic signed [7:0] s8, s8b;
  logic signed [67:0] s68;
  localparam logic signed [3:0] CS4 = 4'sb1000;
  localparam logic signed [7:0] CS8 = 8'sd0;
  localparam logic signed [7:0] CS8B = 8'sb1111_1000;
  localparam logic signed [67:0] CS68 = 68'sd0;
`ifndef NOK
`ifndef NO_INSIDE
  localparam bit KA = (CS4 + CS8) inside {8'sb1111_1?00};
  localparam bit KB = (CS4 + CS8) inside {4'sb1?00};
  localparam bit KC = (CS8B >>> 1) inside {8'sb1111_1?00};
  localparam bit KD = (CS8B >>> 1) inside {4'sb1?00};
  localparam bit KW = (CS4 + CS68) inside {4'sb1?00};
`endif
  localparam bit KAq = (CS4 + CS8) ==? 8'sb1111_1?00;
  localparam bit KBq = (CS4 + CS8) ==? 4'sb1?00;
  localparam bit KCq = (CS8B >>> 1) ==? 8'sb1111_1?00;
  localparam bit KWq = (CS4 + CS68) ==? 4'sb1?00;
  localparam bit KZ = (CS4 + CS8) == 8'sb1111_1000;
  localparam bit KZu = (CS4 + CS8) == 8'b1111_1000;
`endif
  initial begin
    s4 = 4'sb1000; s8 = 8'sd0; s8b = 8'sb1111_1000; s68 = 68'sd0;
    #1;
`ifndef NO_INSIDE
    $display("RI %b %b %b %b %b", (s4 + s8) inside {8'sb1111_1?00}, (s4 + s8) inside {4'sb1?00}, (s8b >>> 1) inside {8'sb1111_1?00}, (s8b >>> 1) inside {4'sb1?00}, (s4 + s68) inside {4'sb1?00});
`ifndef NOK
    $display("KI %b %b %b %b %b", KA, KB, KC, KD, KW);
`endif
`endif
    $display("RQ %b %b %b %b", (s4 + s8) ==? 8'sb1111_1?00, (s4 + s8) ==? 4'sb1?00, (s8b >>> 1) ==? 8'sb1111_1?00, (s4 + s68) ==? 4'sb1?00);
`ifndef NOK
    $display("KQ %b %b %b %b", KAq, KBq, KCq, KWq);
`endif
    $display("RZ %b %b", (s4 + s8) == 8'sb1111_1000, (s4 + s8) == 8'b1111_1000);
    #1 $finish;
  end
endmodule
