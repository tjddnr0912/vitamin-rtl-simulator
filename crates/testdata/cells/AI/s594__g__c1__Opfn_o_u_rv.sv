`timescale 1ns/1ns
package p;
  localparam int PN = 2;
  localparam logic [7:0] PA [0:1] = '{8'hFC, 8'h02};
  function automatic logic [7:0] pf(); return 8'd2; endfunction
endpackage
module t;
  localparam logic [7:0] X = 8'hFC;
  localparam bit C = 1;
  initial #1 $display("RV=%0d", X | p::pf());
  initial #5 $finish;
endmodule
