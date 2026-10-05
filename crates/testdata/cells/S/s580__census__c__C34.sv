`timescale 1ns/1ns
module t;
  logic [3:0] v; wire [7:0] w8;
  function logic f(input logic [3:0] a); f = a inside {4'b1?00, 'x}; endfunction
  function logic [7:0] g(input logic [3:0] a); g = {7'd0, a inside {4'b1?00}} + 8'd1; endfunction
  assign w8 = f(v) + g(v);
  initial begin v = 4'b1100; #1 $display("C34 %h %b %h", w8, f(v), g(v)); v = 4'b0110; #1 $display("C34 %h %b %h", w8, f(v), g(v)); #1 $finish; end
  initial #100 $finish;
endmodule
