package p;
  localparam logic signed [7:0] PX = -8'sd4;
  localparam int PN = 8;
  function automatic int pd(input int a = ((PX + {PN{1'b0}}) == 8'hFC)); return a; endfunction
endpackage
module t;
  localparam int PN = 16;
  localparam int L = p::pd();
  int r;
  initial begin #1 r = p::pd(); $display("L=%0d r=%0d", L, r); end
  initial #100 $finish;
endmodule
