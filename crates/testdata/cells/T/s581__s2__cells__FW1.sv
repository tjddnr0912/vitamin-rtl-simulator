`timescale 1ns/1ns
module sub; logic [35:0] v36 = '0; endmodule
module t;
  sub u();
  initial #1 $display("FW1 %b", u.v36 ==? 'x);
endmodule
