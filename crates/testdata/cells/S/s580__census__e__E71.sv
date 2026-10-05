`timescale 1ns/1ns
module t;
  logic [3:0] v; typedef struct packed {logic [3:0] a; logic [3:0] b;} st; st s;
  initial begin
    s=8'hC1; v=4'b1100; $display("E71 %b", v inside {s.a});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
