`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T X = -4;
  function automatic logic [7:0] f(input logic [7:0] v); return v; endfunction
  localparam L = (f(X) ==? 8'b1111_1?00);
  initial $display("L=%b", L);
endmodule
module t;
  m u();
  initial #5 $finish;
endmodule
