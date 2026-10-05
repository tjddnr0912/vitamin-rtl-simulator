`timescale 1ns/1ns
module t;
  logic [7:0] v;
  initial begin
    v=8'hF4; $display("E86 %b", $signed(v[3:0]) inside {8'sb1111?100});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
