`timescale 1ns/1ns
package p;
  localparam logic signed [7:0] PX = -4;
  localparam int PN = 2;
  localparam bit PC = 1;
  function automatic int pf();
    pf = ((PC ? PX : {PN{1'b1}}) == 8'hFC);
  endfunction
endpackage
module t;
  localparam L = p::pf();
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
