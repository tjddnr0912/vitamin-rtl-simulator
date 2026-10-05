`timescale 1ns/1ns
module t;
  function automatic logic cf(input logic [3:0] a); return a inside {4'b1?00}; endfunction
  localparam logic L = cf(4'b1100);
  initial begin $display("C09 %b", L); #1 $finish; end
  initial #100 $finish;
endmodule
