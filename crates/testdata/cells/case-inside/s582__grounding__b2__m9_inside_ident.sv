module top;
  logic [3:0] inside; logic [3:0] v; int m;
  initial begin inside = 4'b0110; v = 4'b0001; case (v) inside[1:0]: m = 1; default: m = 0; endcase $display("m=%0d", m); #10 $finish; end
endmodule
