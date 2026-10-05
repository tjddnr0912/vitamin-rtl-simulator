module top;
  reg r; wire w; reg [3:0] dv;
  assign #(dv) w = r;
  initial begin dv = 4'bxxx1; r = 1'b0; #5 r = 1'b1; end
  initial begin #5 $strobe("t5 w=%b", w); #1 $display("t6 w=%b", w); end
  initial #100 $finish;
endmodule
