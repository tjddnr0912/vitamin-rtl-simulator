`timescale 1ns/1ns
package p;
  localparam int PN = 2;
  localparam logic [7:0] PA [0:1] = '{8'hFC, 8'h02};
  function automatic logic [7:0] pf(); return 8'd2; endfunction
endpackage
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  if ((C ? X : p::pf()) == 8'hFC) begin : gt initial #1 $display("GI=then"); end else begin : ge initial #1 $display("GI=else"); end
  initial #5 $finish;
endmodule
