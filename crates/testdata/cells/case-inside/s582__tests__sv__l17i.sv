`timescale 1ns/1ns
package p;
  function automatic int f(input logic [3:0] x);
    case (x) inside 4'b1?00: f = 1; [4'd1:4'd3]: f = 2; default: f = 0; endcase
  endfunction
endpackage
module t;
  initial #1000 $finish; // watchdog
  import p::*;
  logic [3:0] vals [0:4] = '{4'b1000, 4'b0010, 4'b0110, 4'b1100, 4'b0011};
  initial begin
    for (int i = 0; i < 5; i++) $display("x=%b f=%0d", vals[i], f(vals[i]));
    $finish;
  end
endmodule
