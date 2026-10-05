`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  function automatic int f(input logic [3:0] x);
    case (x) inside [4'd1:4'd3]: f = 1; 4'b1?00: f = 2; default: f = 0; endcase
  endfunction
  localparam int P1 = f(4'd2);
  initial begin $display("P1=%0d", P1); $finish; end
endmodule
