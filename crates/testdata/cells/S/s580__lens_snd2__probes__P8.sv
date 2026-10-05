`timescale 1ns/1ns
module late;
  real r = 12.0;
  logic [35:0] v = 36'hF_0000_0001;
endmodule
module t;
  late uL();
  logic [35:0] v = 36'hF_0000_0001;
  real lr = 12.0;
  initial begin
    #1;
`ifdef REALQ
    $display("RQ %b", t.uL.r ==? 4'b1?00);
`elsif REALI
    $display("RI %b %b", t.uL.r inside {4'b1?00}, t.uL.r inside {4'b11?0});
`elsif REALL
    $display("RL %b %b", lr inside {4'b1?00}, lr inside {4'b11?0});
`else
    $display("M %b %b %b %b", v inside {'bx1, '1}, v inside {'1, 'bx1}, v inside {32'bx1, 'bx1}, v inside {32'bx1, '0});
`endif
    #1 $finish;
  end
endmodule
