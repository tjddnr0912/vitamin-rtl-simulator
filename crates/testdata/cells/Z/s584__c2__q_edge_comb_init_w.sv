module top;
  logic s, y;
  int nneg = 0, npos = 0, nany = 0;
  always_comb y = (s === 1'b1);
  initial begin
    s = 1'b1;
    #1 $display("t=%0t y=%b neg=%0d pos=%0d any=%0d", $time, y, nneg, npos, nany);
    $finish;
  end
  always @(negedge y) begin nneg++; $display("NEG t=%0t", $time); end
  always @(posedge y) begin npos++; $display("POS t=%0t", $time); end
  always @(y) nany++;
  initial begin wait (y === 1'b0); $display("WAIT0 released t=%0t", $time); end
  initial #100 $finish;
endmodule
