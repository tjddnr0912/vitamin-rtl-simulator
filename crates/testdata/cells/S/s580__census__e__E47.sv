`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b1100; $display("E47 %b", v inside {$unsigned(4'b1?00)});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
