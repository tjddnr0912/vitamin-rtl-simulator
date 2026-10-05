localparam logic signed [7:0] UX = -8'sd4;
localparam int UN = 8;
function automatic int uf(input int a); return (UX + {UN{1'b0}}) == 8'hFC; endfunction
module t;
  localparam int UN = 16;
  int r;
  initial begin #1 r = uf(0); $display("r=%0d", r); end
  initial #100 $finish;
endmodule
