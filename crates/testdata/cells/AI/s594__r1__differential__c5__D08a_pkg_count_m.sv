package p;
  localparam N = 16;
endpackage
module t;
  import p::*;
  localparam L1 = ((({p::N{1'b0}} + 8'd255 + 8'd1) >> 1) == 8'd128);
  localparam L2 = ((({N{1'b0}} + 8'd255 + 8'd1) >> 1) == 8'd128);
  initial $display("R: L1=%0d L2=%0d", L1, L2);
  initial #10 $finish;
endmodule
