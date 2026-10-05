module top;
  logic [3:0] v = 4'b1000; int m;
  initial begin casez (v) inside 4'b1?00: m = 1; default: m = 0; endcase $display("m=%0d", m); #10 $finish; end
endmodule
