`timescale 1ns/1ns
module t;
  logic [3:0] v; logic [1:0] p;
  initial begin
    v=4'b1100; p=2'b11; $display("E82 %b", v inside {{p, 2'b?0}});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
