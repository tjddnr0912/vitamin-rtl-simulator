localparam logic signed [7:0] UX = -8'sd4;
localparam int UN = 8;
function automatic int uf(input int a); return (UX + {UN{1'b0}}) == 8'hFC; endfunction
module t;
  localparam int UN = 16;
  localparam int L = uf(0);
  int r;
  initial begin #1 r = uf(0); $display("L=%0d r=%0d", L, r); end
  initial #100 $finish;
endmodule
