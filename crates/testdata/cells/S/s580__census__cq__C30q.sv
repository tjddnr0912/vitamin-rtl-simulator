`timescale 1ns/1ns
module t;
  logic [3:0] v;
  function automatic logic f(input logic [3:0] a); logic r; r = 1'b0; if (a ==? 4'b1?00) r = 1'b1; return r; endfunction
  localparam logic LC = f(4'b1000);
  initial begin v=4'b1100; $display("C30q %b %b", f(v), LC); #1 $finish; end
  initial #100 $finish;
endmodule
