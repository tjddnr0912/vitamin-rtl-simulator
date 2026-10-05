package p; typedef struct packed { logic a; logic [4:0] b; } st_t; typedef logic [5:0] u;
  localparam u   Q = 6'd5;
  localparam st_t R = 6'd6;
  localparam bit V = 1'b1;
endpackage
module tb; import p::*; initial begin $display("DIGEST=%0d", Q+R+V); #1 $finish; end endmodule