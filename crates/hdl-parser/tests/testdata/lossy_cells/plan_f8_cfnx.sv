module t; function automatic logic [3:0] f(); f = 4'b1x01; endfunction
  localparam logic [3:0] P = f(); logic [P:0] w;
  initial begin #1 $display("A p=%b bw=%0d", P, $bits(w)); end
  initial #100 $finish;
endmodule
