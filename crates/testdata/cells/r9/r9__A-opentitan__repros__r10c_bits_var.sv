module t;
  logic [39:0] v;
  localparam int W = $bits(v);
  initial begin #1 $display("A W=%0d", W); $finish; end
endmodule
