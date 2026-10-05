`timescale 1ns/1ns
package p;
  localparam int PN = 2;
  localparam logic [7:0] PA [0:1] = '{8'hFC, 8'h02};
  function automatic logic [7:0] pf(); return 8'd2; endfunction
endpackage
module t;
  localparam logic signed [7:0] X = -4;
  localparam bit C = 1;
  case (X | p::pf()) 8'hFE: begin : gk initial #1 $display("GC=item"); end default: begin : gd initial #1 $display("GC=def"); end endcase
  initial #5 $finish;
endmodule
