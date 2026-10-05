`timescale 1ns/1ns
module m #(parameter type T = logic [7:0]);
  localparam T X = -4;
  function automatic logic [7:0] f(input logic [7:0] v); return v; endfunction
  logic [3:0] arr [0:(f(X) ==? 8'b1111_1?00)+2];
  initial $display("asz=%0d", $size(arr));
endmodule
module t;
  m #(.T(logic signed [7:0])) u();
  initial #5 $finish;
endmodule
