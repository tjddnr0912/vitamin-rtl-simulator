package r; typedef struct packed { logic [11:0] a; logic [3:0] b; } rs_t; endpackage
module tb; typedef struct packed { logic [3:0] a; logic [3:0] b; } ls_t; ls_t rs_t; import r::rs_t;
  initial begin rs_t = 8'h34; $display("DIGEST=%h %h", rs_t.a, rs_t.b); #1 $finish; end
endmodule