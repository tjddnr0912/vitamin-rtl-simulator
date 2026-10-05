`timescale 1ns/1ns
module t;
  function automatic logic signed [3:0] fxs(input int a); logic signed [3:0] t; return t - 4'sd4; endfunction
  localparam logic signed [3:0] PA = fxs(2);
  initial begin #1 $display("PA=%0d PAb=%b", PA, PA); #5 $finish; end
endmodule
