`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fr(input int a); if (a > 5) fr = 4'd1; endfunction
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: (fr(2) inside {4'b0?00})+3];
  initial #1 $display("ps=%b", ps);
  initial #5 $finish;
endmodule
