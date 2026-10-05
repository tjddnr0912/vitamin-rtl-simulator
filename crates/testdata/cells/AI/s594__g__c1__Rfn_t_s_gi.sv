`timescale 1ns/1ns
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  function automatic int f2(input int a); return a; endfunction
  if ((C ? X : {f2(2){1'b0}}) == 8'hFC) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #5 $finish;
endmodule
