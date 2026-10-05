`timescale 1ns/1ns
module t;
  initial #1000 $finish; // watchdog
  logic [3:0] x; logic [31:0] y;
  function automatic int fn(input logic [3:0] a);
    case (a) inside 4'b1?00: fn = 1; [4'd1:4'd3]: fn = 2; default: fn = 0; endcase
  endfunction
  assign y = fn(x);
  logic [3:0] vals [0:4] = '{4'b1000, 4'b0010, 4'b0110, 4'b1100, 4'b0011};
  initial begin
    for (int i = 0; i < 5; i++) begin x = vals[i]; #1 $display("x=%b y=%0d", x, y); end
    $finish;
  end
endmodule
