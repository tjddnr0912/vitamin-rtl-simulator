`timescale 1ns/1ns
module t;
  function automatic logic [3:0] fx(input int a); logic [3:0] t; return t; endfunction
  wire [7:0] s = 8'hA5;
  wire [7:0] ps = s[0 +: ($clog2(fx(2) + 4'd1) inside {32'b0000_0000_0000_0000_0000_0000_0000_000?})+3];
  initial #1 $display("ps=%b", ps);
  initial #5 $finish;
endmodule
