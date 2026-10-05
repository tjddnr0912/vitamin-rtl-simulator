`timescale 1ns/1ns
module t;
  typedef struct packed {logic [3:0] a; logic [3:0] b;} st; st s;
  initial begin
    s=8'hC1; $display("E72 %b", s.a inside {4'b1?00});
    #1 $finish;
  end
  initial #100 $finish;
endmodule
