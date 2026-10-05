`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic logic [7:0] fg(input int a); logic [7:0] r; r = 8'd0; fg = r + a; endfunction
  reg r = 0;
  initial #10 r = 1;
  wire w;
  assign #((X + {N{1'b0}}) + fg(2)) w = r;
  always @(w) $display("w=%b t=%0t", w, $time);
  initial #600 $finish;
endmodule
