`timescale 1ns/1ns
package p;
  function automatic int f(input logic [3:0] x);
    case (x) inside 4'b1?00: f = 1; [4'd1:4'd3]: f = 2; default: f = 0; endcase
  endfunction
endpackage
module t;
  initial #1000 $finish; // watchdog
  initial begin
    $display("f=%0d", p::f(4'b1000));
    $finish;
  end
endmodule
