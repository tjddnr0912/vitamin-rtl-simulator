`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fx(input int a); logic [3:0] t; return t; endfunction
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: ({fx(2), 4'b0000} inside {8'b0?00_0000})+3];
  initial #1 $display("ps=%b", ps);
  initial #5 $finish;
endmodule
