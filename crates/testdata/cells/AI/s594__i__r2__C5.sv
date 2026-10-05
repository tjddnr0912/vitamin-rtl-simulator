module t;
  localparam logic signed [7:0] X = -8'sd4;
  localparam int N = 8;
  function automatic logic [7:0] fl(input int a); logic [7:0] r; fl = r; endfunction
  logic [7:0] v [0:1];
  localparam int L = fl(0) + ((AS[0] + 8'd0) == 8'hFC);
  localparam logic signed [7:0] AS [0:1] = '{-8'sd4, -8'sd4};
  initial $display("L=%0d", L);
  initial #100 $finish;
endmodule
