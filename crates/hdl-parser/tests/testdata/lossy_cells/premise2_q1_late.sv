module leaf #(parameter int P = 40); logic [69:0] x; endmodule
module t;
  leaf u();
  for (genvar gi = 0; gi < 1; gi++) begin : gb localparam int K = 40; end
  logic [u.P-1:0] v1 = 'x;
  logic [$bits(u.x)-1:0] v2 = 'z;
  logic [gb[0].K-1:0] v3 = 'x;
  initial begin #1 $display("A v1=%h v2=%h v3=%h", v1, v2, v3); end
  initial #100 $finish;
endmodule
