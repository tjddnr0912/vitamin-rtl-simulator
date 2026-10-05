module t;
  localparam logic signed [7:0] X = -4;
  localparam int N = 2;
  function automatic int fz(input int a); fz = a; endfunction
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r + a; endfunction
  localparam Q = fl(0);
  if ((X + 2'b00) == (Q + 8'd252)) begin : GA initial #2 $display("R: GI=then"); end
  else begin : GB initial #2 $display("R: GI=else"); end
  initial #40 $finish;
endmodule
