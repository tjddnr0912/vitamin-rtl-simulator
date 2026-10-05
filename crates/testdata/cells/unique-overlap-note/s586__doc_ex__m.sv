module m;
  logic [1:0] r = 2'b11; int y;
  initial begin
    #1 unique casez (r)
      2'b?1: y = 1;   // runs: the first match
      2'b1?: y = 2;   // also matches r = 2'b11; not reported
    endcase
    $display("y=%0d", y);
  end
endmodule
