`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  function automatic logic [7:0] fg(input int a); logic [7:0] r; r = 8'd0; fg = r + a; endfunction
  wire w;
  assign #((X + 2'b00) + fg(2)) w = 1'b1;
  always @(w) $display("w=%b t=%0t", w, $time);
  initial #300 $finish;
endmodule
