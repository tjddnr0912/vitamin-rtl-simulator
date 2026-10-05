`timescale 1ns/1ns
package p;
  localparam logic signed [7:0] PX = -4;
  localparam logic [7:0] PA [0:1] = '{8'hFC, 8'h04};
  localparam bit PC = 1;
  function automatic int pf();
    pf = (((PC ? PX : PA[1]) + 8'd4) == 8'h00);
  endfunction
endpackage
module t;
  localparam logic [15:0] PA [0:1] = '{16'h1, 16'h2};
  localparam L = p::pf();
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
