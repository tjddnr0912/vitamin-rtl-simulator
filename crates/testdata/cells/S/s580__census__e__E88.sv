`timescale 1ns/1ns
module t;
  logic [3:0] v;
  initial begin
    v=4'b1100; $display("E88 %b", v inside {4'd1/4'd0});
    v=4'b1100; $display("E89 %b", v inside {4'd8/4'd2, 4'd12});
    v=4'b0100; $display("E90 %b", v inside {4'd8/4'd2});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
