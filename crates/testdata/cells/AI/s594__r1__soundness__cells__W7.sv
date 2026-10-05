module sub #(parameter P = 8'd1);
  initial $display("P=%0d B=%0d", P, $bits(P));
endmodule
module t;
  localparam int N = 9;
  sub #(.P(16'({N{8'hFF}}))) u();
  initial #100 $finish;
endmodule
