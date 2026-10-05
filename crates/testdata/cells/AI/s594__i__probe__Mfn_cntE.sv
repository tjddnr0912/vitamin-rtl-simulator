`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] PX = -4;
  localparam int PN = 2;
  localparam bit PC = 1;
  function automatic int mf();
    mf = ((PC ? PX : {PN{1'b1}}) == 8'hFC);
  endfunction
  localparam L = mf();
  initial #1 $display("L=%0d", L);
  initial #5 $finish;
endmodule
