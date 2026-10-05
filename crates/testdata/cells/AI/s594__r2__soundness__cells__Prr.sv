package p;
  localparam logic signed [7:0] PX = -8'sd4;
  localparam int PN = 8;
  function automatic int pf(input int a); return (PX + {PN{1'b0}}) == 8'hFC; endfunction
endpackage
module t;
  localparam int PN = 16;
  int r;
  initial begin #1 r = p::pf(0); $display("r=%0d", r); end
  initial #100 $finish;
endmodule
