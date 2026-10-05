module top;
  logic [3:0] v, lo, hi; int m;
  always_comb begin
    case (v) inside [lo:hi]: m = 1; 4'b1?00: m = 2; default: m = 0; endcase
  end
  initial begin
    v = 4'd5; lo = 4'd1; hi = 4'd3;
    #1 $display("t1 m=%0d", m);
    hi = 4'd6;
    #1 $display("t2 m=%0d (hi changed)", m);
    lo = 4'd6;
    #1 $display("t3 m=%0d (lo changed)", m);
    v = 4'd12;
    #1 $display("t4 m=%0d (v changed)", m);
    #10 $finish;
  end
endmodule
