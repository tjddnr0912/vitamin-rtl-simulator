`timescale 1ns/1ns
module t;
  logic [3:0] v; logic r;
  task automatic tk(input logic [3:0] a, output logic o); o = a inside {4'b1?00}; endtask
  initial begin v=4'b1100; tk(v, r); $display("C07 %b", r); #1 $finish; end
  initial #100 $finish;
endmodule
