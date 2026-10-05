`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b1010; $display("E11 %b", v inside {'z});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
