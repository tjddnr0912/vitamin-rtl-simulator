`timescale 1ns/1ns
module t;
  logic [15:0] s16;
  initial begin
    s16="ab"; $display("E60 %b", s16 inside {"ab"});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
